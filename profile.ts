import { createClient } from "redis";

type WindowStats = {
  commands: number;
  requests: number;
  cmdRps: number;
  reqRps: number;
  parseMs: number;
  executeMs: number;
  encodeMs: number;
  writeMs: number;
  totalStageMs: number;
  longRequests: number;
};

type CommandStats = {
  command: string;
  count: number;
  totalMs: number;
  avgUs: number;
  maxUs: number;
  slowCount: number;
};

type RequestCommandStats = {
  command: string;
  count: number;
  totalMs: number;
  avgUs: number;
  maxUs: number;
  slowCount: number;
  parseMs: number;
  executeMs: number;
  encodeMs: number;
  hotStage: "parse" | "execute" | "encode";
  hotPct: number;
};

type SlowRequestSample = {
  command: string;
  totalUs: number;
  parseUs: number;
  executeUs: number;
  encodeUs: number;
  bottleneck: "parse" | "execute" | "encode";
};

type Scenario = {
  command: string;
  setup?: (client: ReturnType<typeof createClient>) => Promise<void>;
  run: (client: ReturnType<typeof createClient>) => Promise<void>;
};

type ScenarioResult = {
  command: string;
  operationCount: number;
  busiestWindow: WindowStats;
  measuredWindow: WindowStats;
  window: WindowStats;
  commandStats?: CommandStats;
  requestStats?: RequestCommandStats;
  slowSamples: SlowRequestSample[];
  stagePerOpUs: {
    parse: number;
    execute: number;
    encode: number;
    write: number;
    total: number;
  };
};

const SERVER_BIN = "target/release/justkv-server";
const PORT_START = 6400;
const PROFILE_INTERVAL_SECS = 2;
const SCENARIO_ITERATIONS = 20_000;
const SCENARIO_CONCURRENCY = 200;
const BURST_ITERATIONS = 80_000;
const BURST_CONCURRENCY = 600;
const LONG_REQUEST_THRESHOLD_MS = 1;
const LONG_REQUEST_SAMPLES = 12;

const sleep = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

async function waitForServer(url: string, timeoutMs = 30_000): Promise<void> {
  const startedAt = Date.now();
  while (Date.now() - startedAt < timeoutMs) {
    const client = createClient({ url });
    try {
      await client.connect();
      await client.ping();
      await client.disconnect();
      return;
    } catch {
      try {
        await client.disconnect();
      } catch {
        // ignore disconnect errors during retries
      }
      await sleep(200);
    }
  }
  throw new Error(`Server did not become ready within ${timeoutMs}ms`);
}

async function runBatched(
  totalOps: number,
  concurrency: number,
  fn: (i: number) => Promise<void>
): Promise<void> {
  for (let i = 0; i < totalOps; i += concurrency) {
    const batch: Promise<void>[] = [];
    for (let j = 0; j < concurrency && i + j < totalOps; j++) {
      batch.push(fn(i + j));
    }
    await Promise.all(batch);
  }
}

function parseProfiler(stderr: string): {
  windows: WindowStats[];
  commands: CommandStats[];
  requestCommands: RequestCommandStats[];
  slowRequests: SlowRequestSample[];
} {
  const windows: WindowStats[] = [];
  const commands: CommandStats[] = [];
  const requestCommands: RequestCommandStats[] = [];
  const slowRequests: SlowRequestSample[] = [];

  const lines = stderr
    .split("\n")
    .map((line) => line.trim())
    .filter(Boolean);

  for (const line of lines) {
    const windowMatch = line.match(
      /^\[latency-profiler\].*commands=(\d+)(?:\s+requests=(\d+))?(?:\s+cmd_rps=([0-9.]+))?(?:\s+req_rps=([0-9.]+))?\s+parse=([0-9.]+)ms\s+execute=([0-9.]+)ms\s+encode=([0-9.]+)ms\s+write=([0-9.]+)ms(?:\s+total_stage=([0-9.]+)ms)?(?:\s+long_requests=(\d+))?/
    );
    if (windowMatch) {
      windows.push({
        commands: Number(windowMatch[1]),
        requests: windowMatch[2] ? Number(windowMatch[2]) : Number(windowMatch[1]),
        cmdRps: windowMatch[3]
          ? Number(windowMatch[3])
          : Number(windowMatch[1]) / PROFILE_INTERVAL_SECS,
        reqRps: windowMatch[4]
          ? Number(windowMatch[4])
          : (windowMatch[2] ? Number(windowMatch[2]) : Number(windowMatch[1])) /
            PROFILE_INTERVAL_SECS,
        parseMs: Number(windowMatch[5]),
        executeMs: Number(windowMatch[6]),
        encodeMs: Number(windowMatch[7]),
        writeMs: Number(windowMatch[8]),
        totalStageMs: windowMatch[9]
          ? Number(windowMatch[9])
          : Number(windowMatch[5]) +
            Number(windowMatch[6]) +
            Number(windowMatch[7]) +
            Number(windowMatch[8]),
        longRequests: windowMatch[10] ? Number(windowMatch[10]) : 0,
      });
      continue;
    }

    const commandMatch = line.match(
      /^\[latency-profiler\]\s+cmd=([^\s]+) count=(\d+) total=([0-9.]+)ms avg=([0-9.]+)us max=([0-9.]+)us slow=(\d+)/
    );
    if (commandMatch) {
      commands.push({
        command: commandMatch[1],
        count: Number(commandMatch[2]),
        totalMs: Number(commandMatch[3]),
        avgUs: Number(commandMatch[4]),
        maxUs: Number(commandMatch[5]),
        slowCount: Number(commandMatch[6]),
      });
      continue;
    }

    const requestCommandMatch = line.match(
      /^\[latency-profiler\]\s+req_cmd=([^\s]+) count=(\d+) total=([0-9.]+)ms avg=([0-9.]+)us max=([0-9.]+)us slow=(\d+) parse=([0-9.]+)ms execute=([0-9.]+)ms encode=([0-9.]+)ms hot=(parse|execute|encode) hot_pct=([0-9.]+)/
    );
    if (requestCommandMatch) {
      requestCommands.push({
        command: requestCommandMatch[1],
        count: Number(requestCommandMatch[2]),
        totalMs: Number(requestCommandMatch[3]),
        avgUs: Number(requestCommandMatch[4]),
        maxUs: Number(requestCommandMatch[5]),
        slowCount: Number(requestCommandMatch[6]),
        parseMs: Number(requestCommandMatch[7]),
        executeMs: Number(requestCommandMatch[8]),
        encodeMs: Number(requestCommandMatch[9]),
        hotStage: requestCommandMatch[10] as RequestCommandStats["hotStage"],
        hotPct: Number(requestCommandMatch[11]),
      });
      continue;
    }

    const slowRequestMatch = line.match(
      /^\[latency-profiler\]\s+slow_req cmd=([^\s]+) total=([0-9.]+)us parse=([0-9.]+)us execute=([0-9.]+)us encode=([0-9.]+)us bottleneck=(parse|execute|encode)/
    );
    if (slowRequestMatch) {
      slowRequests.push({
        command: slowRequestMatch[1],
        totalUs: Number(slowRequestMatch[2]),
        parseUs: Number(slowRequestMatch[3]),
        executeUs: Number(slowRequestMatch[4]),
        encodeUs: Number(slowRequestMatch[5]),
        bottleneck: slowRequestMatch[6] as SlowRequestSample["bottleneck"],
      });
    }
  }

  return { windows, commands, requestCommands, slowRequests };
}

function perOpUs(window: WindowStats, opCount: number) {
  const divisor = Math.max(opCount, 1);
  const parse = (window.parseMs * 1000) / divisor;
  const execute = (window.executeMs * 1000) / divisor;
  const encode = (window.encodeMs * 1000) / divisor;
  const write = (window.writeMs * 1000) / divisor;
  return {
    parse,
    execute,
    encode,
    write,
    total: parse + execute + encode + write,
  };
}

async function runScenario(
  scenario: Scenario,
  port: number
): Promise<ScenarioResult> {
  const env = {
    ...process.env,
    JUSTKV_PROFILE: "1",
    JUSTKV_PROFILE_INTERVAL_SECS: String(PROFILE_INTERVAL_SECS),
    JUSTKV_PROFILE_SLOW_MS: "1",
    JUSTKV_PROFILE_LONG_MS: String(LONG_REQUEST_THRESHOLD_MS),
    JUSTKV_PROFILE_SLOW_SAMPLES: String(LONG_REQUEST_SAMPLES),
  };

  const server = Bun.spawn([SERVER_BIN, "--port", String(port)], {
    stdout: "ignore",
    stderr: "pipe",
    env,
  });
  const stderrPromise = new Response(server.stderr).text();

  const url = `redis://127.0.0.1:${port}`;
  await waitForServer(url);

  const client = createClient({ url });
  await client.connect();

  if (scenario.setup) {
    await scenario.setup(client);
  }

  await sleep((PROFILE_INTERVAL_SECS * 1000) + 250);
  await scenario.run(client);
  await sleep((PROFILE_INTERVAL_SECS * 1000) + 250);

  await client.disconnect();
  server.kill();
  await server.exited;

  const stderr = await stderrPromise;
  const parsed = parseProfiler(stderr);
  const measuredWindow = [...parsed.windows].reverse().find((w) => w.commands > 0);
  if (!measuredWindow) {
    throw new Error(`No profiler windows captured for ${scenario.command}`);
  }

  const busiestWindow = parsed.windows.reduce((best, current) => {
    if (current.requests > best.requests) {
      return current;
    }
    return best;
  }, measuredWindow);

  const commandStats = [...parsed.commands]
    .reverse()
    .find((c) => c.command === scenario.command);
  const requestStats = [...parsed.requestCommands]
    .reverse()
    .find((c) => c.command === scenario.command);
  const operationCount =
    commandStats?.count ?? requestStats?.count ?? busiestWindow.requests;

  return {
    command: scenario.command,
    operationCount,
    busiestWindow,
    measuredWindow,
    window: measuredWindow,
    commandStats,
    requestStats,
    slowSamples: parsed.slowRequests,
    stagePerOpUs: perOpUs(busiestWindow, operationCount),
  };
}

async function runMixedBurstScenario(client: ReturnType<typeof createClient>) {
  await runBatched(BURST_ITERATIONS, BURST_CONCURRENCY, async (i) => {
    const lane = i % 8;
    if (lane === 0) {
      await client.set(`prof:burst:set:${i}`, `value-${i}`);
      return;
    }
    if (lane === 1) {
      await client.get(`prof:burst:get:${i % SCENARIO_ITERATIONS}`);
      return;
    }
    if (lane === 2) {
      await client.incr(`prof:burst:incr:${i % 2048}`);
      return;
    }
    if (lane === 3) {
      await client.hSet(`prof:burst:hash:${i % 256}`, `field:${i}`, `value:${i}`);
      return;
    }
    if (lane === 4) {
      await client.sAdd(`prof:burst:setbag:${i % 128}`, `member:${i}`);
      return;
    }
    if (lane === 5) {
      await client.lPush(`prof:burst:queue:${i % 64}`, `item:${i}`);
      return;
    }
    if (lane === 6) {
      await client.lRange("prof:burst:hotlist", 0, 2000);
      return;
    }
    await client.expire(`prof:burst:get:${i % SCENARIO_ITERATIONS}`, 60);
  });
}

const scenarios: Scenario[] = [
  {
    command: "SET",
    run: async (client) => {
      await runBatched(SCENARIO_ITERATIONS, SCENARIO_CONCURRENCY, async (i) => {
        await client.set(`prof:set:${i}`, `value-${i}`);
      });
    },
  },
  {
    command: "GET",
    setup: async (client) => {
      await runBatched(SCENARIO_ITERATIONS, SCENARIO_CONCURRENCY, async (i) => {
        await client.set(`prof:get:${i}`, `value-${i}`);
      });
    },
    run: async (client) => {
      await runBatched(SCENARIO_ITERATIONS, SCENARIO_CONCURRENCY, async (i) => {
        await client.get(`prof:get:${i}`);
      });
    },
  },
  {
    command: "INCR",
    run: async (client) => {
      await runBatched(SCENARIO_ITERATIONS, SCENARIO_CONCURRENCY, async (i) => {
        await client.incr(`prof:incr:${i % 1000}`);
      });
    },
  },
  {
    command: "HSET",
    run: async (client) => {
      await runBatched(SCENARIO_ITERATIONS, SCENARIO_CONCURRENCY, async (i) => {
        await client.hSet(`prof:hset:${i % 1000}`, `field:${i}`, `value:${i}`);
      });
    },
  },
  {
    command: "SADD",
    run: async (client) => {
      await runBatched(SCENARIO_ITERATIONS, SCENARIO_CONCURRENCY, async (i) => {
        await client.sAdd(`prof:sadd:${i % 1000}`, `member:${i}`);
      });
    },
  },
  {
    command: "LPUSH",
    run: async (client) => {
      await runBatched(SCENARIO_ITERATIONS, SCENARIO_CONCURRENCY, async (i) => {
        await client.lPush(`prof:lpush:${i % 1000}`, `item:${i}`);
      });
    },
  },
  {
    command: "LRANGE",
    setup: async (client) => {
      await client.del("prof:lrange:list");
      await runBatched(1_000, 100, async (i) => {
        await client.lPush("prof:lrange:list", `item:${i}`);
      });
    },
    run: async (client) => {
      await runBatched(SCENARIO_ITERATIONS, SCENARIO_CONCURRENCY, async () => {
        await client.lRange("prof:lrange:list", 0, 99);
      });
    },
  },
  {
    command: "EXPIRE",
    setup: async (client) => {
      await runBatched(SCENARIO_ITERATIONS, SCENARIO_CONCURRENCY, async (i) => {
        await client.set(`prof:expire:${i}`, "1");
      });
    },
    run: async (client) => {
      await runBatched(SCENARIO_ITERATIONS, SCENARIO_CONCURRENCY, async (i) => {
        await client.expire(`prof:expire:${i}`, 120);
      });
    },
  },
  {
    command: "MIXED",
    setup: async (client) => {
      await runBatched(SCENARIO_ITERATIONS, SCENARIO_CONCURRENCY, async (i) => {
        await client.set(`prof:burst:get:${i}`, `value-${i}`);
      });
      await client.del("prof:burst:hotlist");
      await runBatched(25_000, 500, async (i) => {
        await client.lPush("prof:burst:hotlist", `item:${i}`);
      });
    },
    run: async (client) => {
      await runMixedBurstScenario(client);
    },
  },
];

async function main() {
  console.log("Building release binary...");
  const build = Bun.spawn(["cargo", "build", "-p", "justkv-server", "--release"], {
    stdout: "inherit",
    stderr: "inherit",
  });
  const buildExit = await build.exited;
  if (buildExit !== 0) {
    throw new Error("Build failed");
  }

  const results: ScenarioResult[] = [];
  for (let i = 0; i < scenarios.length; i++) {
    const scenario = scenarios[i];
    const port = PORT_START + i;
    console.log(`Profiling ${scenario.command} on port ${port}...`);
    const result = await runScenario(scenario, port);
    results.push(result);
  }

  const summary = results.map((result) => ({
    command: result.command,
    operations: result.operationCount,
    busiest_window_commands: result.busiestWindow.commands,
    busiest_window_requests: result.busiestWindow.requests,
    busiest_window_cmd_rps: Number(result.busiestWindow.cmdRps.toFixed(1)),
    busiest_window_req_rps: Number(result.busiestWindow.reqRps.toFixed(1)),
    busiest_window_long_requests: result.busiestWindow.longRequests,
    parse_us_per_op: Number(result.stagePerOpUs.parse.toFixed(3)),
    execute_us_per_op: Number(result.stagePerOpUs.execute.toFixed(3)),
    encode_us_per_op: Number(result.stagePerOpUs.encode.toFixed(3)),
    write_us_per_op: Number(result.stagePerOpUs.write.toFixed(3)),
    total_us_per_op: Number(result.stagePerOpUs.total.toFixed(3)),
    command_execute_avg_us: result.commandStats
      ? Number(result.commandStats.avgUs.toFixed(3))
      : null,
    command_execute_max_us: result.commandStats
      ? Number(result.commandStats.maxUs.toFixed(3))
      : null,
    command_long_execute_count: result.commandStats
      ? result.commandStats.slowCount
      : null,
    request_profile: result.requestStats
      ? {
          avg_us: Number(result.requestStats.avgUs.toFixed(3)),
          max_us: Number(result.requestStats.maxUs.toFixed(3)),
          long_count: result.requestStats.slowCount,
          parse_ms_total: Number(result.requestStats.parseMs.toFixed(3)),
          execute_ms_total: Number(result.requestStats.executeMs.toFixed(3)),
          encode_ms_total: Number(result.requestStats.encodeMs.toFixed(3)),
          hot_stage: result.requestStats.hotStage,
          hot_stage_pct: Number(result.requestStats.hotPct.toFixed(1)),
        }
      : null,
    slow_request_samples: result.slowSamples.slice(0, 5).map((sample) => ({
      cmd: sample.command,
      total_us: Number(sample.totalUs.toFixed(3)),
      parse_us: Number(sample.parseUs.toFixed(3)),
      execute_us: Number(sample.executeUs.toFixed(3)),
      encode_us: Number(sample.encodeUs.toFixed(3)),
      bottleneck: sample.bottleneck,
    })),
  }));

  await Bun.write("profile-results.json", JSON.stringify(summary, null, 2));

  console.log("\nPer-command stage breakdown in busiest load window (microseconds per op):");
  for (const item of summary) {
    console.log(
      `${item.command.padEnd(8)} parse=${item.parse_us_per_op.toFixed(3)} ` +
        `exec=${item.execute_us_per_op.toFixed(3)} ` +
        `encode=${item.encode_us_per_op.toFixed(3)} ` +
        `write=${item.write_us_per_op.toFixed(3)} ` +
        `total=${item.total_us_per_op.toFixed(3)} ` +
        `req_rps=${item.busiest_window_req_rps.toFixed(1)} ` +
        `long=${item.busiest_window_long_requests}`
    );

    if (item.request_profile) {
      console.log(
        `         long_request_hot_stage=${item.request_profile.hot_stage} ` +
          `(${item.request_profile.hot_stage_pct.toFixed(1)}%) ` +
          `long_count=${item.request_profile.long_count}`
      );
    }
    if (item.slow_request_samples.length > 0) {
      const topSlow = item.slow_request_samples[0];
      console.log(
        `         worst_slow_request cmd=${topSlow.cmd} total=${topSlow.total_us.toFixed(3)}us ` +
          `parse=${topSlow.parse_us.toFixed(3)}us exec=${topSlow.execute_us.toFixed(3)}us ` +
          `encode=${topSlow.encode_us.toFixed(3)}us bottleneck=${topSlow.bottleneck}`
      );
    }
  }
  console.log("\nSaved JSON report to profile-results.json");
}

main().catch((error) => {
  console.error("Profiling failed:", error);
  process.exit(1);
});
