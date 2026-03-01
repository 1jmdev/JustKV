import { createClient } from "redis";

type WindowStats = {
  commands: number;
  parseMs: number;
  executeMs: number;
  encodeMs: number;
  writeMs: number;
};

type CommandStats = {
  command: string;
  count: number;
  totalMs: number;
  avgUs: number;
  maxUs: number;
  slowCount: number;
};

type Scenario = {
  command: string;
  setup?: (client: ReturnType<typeof createClient>) => Promise<void>;
  run: (client: ReturnType<typeof createClient>) => Promise<void>;
};

type ScenarioResult = {
  command: string;
  operationCount: number;
  window: WindowStats;
  commandStats?: CommandStats;
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

function parseProfiler(stderr: string): { windows: WindowStats[]; commands: CommandStats[] } {
  const windows: WindowStats[] = [];
  const commands: CommandStats[] = [];

  const lines = stderr
    .split("\n")
    .map((line) => line.trim())
    .filter(Boolean);

  for (const line of lines) {
    const windowMatch = line.match(
      /commands=(\d+) parse=([0-9.]+)ms execute=([0-9.]+)ms encode=([0-9.]+)ms write=([0-9.]+)ms/
    );
    if (windowMatch) {
      windows.push({
        commands: Number(windowMatch[1]),
        parseMs: Number(windowMatch[2]),
        executeMs: Number(windowMatch[3]),
        encodeMs: Number(windowMatch[4]),
        writeMs: Number(windowMatch[5]),
      });
      continue;
    }

    const commandMatch = line.match(
      /cmd=([^\s]+) count=(\d+) total=([0-9.]+)ms avg=([0-9.]+)us max=([0-9.]+)us slow=(\d+)/
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
    }
  }

  return { windows, commands };
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

  const commandStats = [...parsed.commands]
    .reverse()
    .find((c) => c.command === scenario.command);
  const operationCount = commandStats?.count ?? measuredWindow.commands;

  return {
    command: scenario.command,
    operationCount,
    window: measuredWindow,
    commandStats,
    stagePerOpUs: perOpUs(measuredWindow, operationCount),
  };
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
  }));

  await Bun.write("profile-results.json", JSON.stringify(summary, null, 2));

  console.log("\nPer-command stage breakdown (microseconds per op):");
  for (const item of summary) {
    console.log(
      `${item.command.padEnd(8)} parse=${item.parse_us_per_op.toFixed(3)} ` +
        `exec=${item.execute_us_per_op.toFixed(3)} ` +
        `encode=${item.encode_us_per_op.toFixed(3)} ` +
        `write=${item.write_us_per_op.toFixed(3)} ` +
        `total=${item.total_us_per_op.toFixed(3)}`
    );
  }
  console.log("\nSaved JSON report to profile-results.json");
}

main().catch((error) => {
  console.error("Profiling failed:", error);
  process.exit(1);
});
