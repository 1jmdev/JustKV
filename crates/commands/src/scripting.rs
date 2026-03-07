use crate::dispatcher;
use crate::util::{int_error, parse_i64_bytes, wrong_args, Args};
use engine::store::Store;
use mlua::{HookTriggers, Lua, Table, Value, Variadic, VmState};
use parking_lot::Mutex;
use protocol::types::{BulkData, RespFrame};
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};
use std::thread;
use tokio::sync::{Semaphore, SemaphorePermit, TryAcquireError};
use types::value::CompactArg;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ScriptDebugMode {
    No,
    Yes,
    Sync,
}

struct RunningScript {
    kill_requested: Arc<AtomicBool>,
    performed_write: Arc<AtomicBool>,
}

struct ScriptRuntime {
    debug_mode: ScriptDebugMode,
    running: Option<RunningScript>,
}

struct ScriptRuntimeSync {
    state: Mutex<ScriptRuntime>,
    permit: Semaphore,
}

struct ScriptExecutionGuard {
    kill_requested: Arc<AtomicBool>,
    performed_write: Arc<AtomicBool>,
    debug_mode: ScriptDebugMode,
    _permit: SemaphorePermit<'static>,
}

#[derive(Clone)]
struct LuaCallContext {
    store: Store,
    readonly: bool,
    wrote: Arc<AtomicBool>,
    kill_requested: Arc<AtomicBool>,
    debug_mode: ScriptDebugMode,
    arg_buf: RefCell<Vec<CompactArg>>,
}

struct LuaRuntime {
    lua: Lua,
    compiled_scripts: HashMap<Vec<u8>, mlua::RegistryKey>,
    fast_scripts: HashMap<Vec<u8>, FastScript>,
    keys_table: mlua::RegistryKey,
    argv_table: mlua::RegistryKey,
}

#[derive(Clone, Copy)]
enum FastScript {
    SetKey1Arg1ReturnArg1,
}

impl LuaRuntime {
    fn new() -> Result<Self, String> {
        let lua = Lua::new();

        lua.set_hook(
            HookTriggers {
                every_nth_instruction: Some(1000),
                ..HookTriggers::default()
            },
            |lua, _debug| {
                if let Some(context) = lua.app_data_ref::<LuaCallContext>() {
                    if context.debug_mode != ScriptDebugMode::Sync
                        && context.kill_requested.load(Ordering::Relaxed)
                    {
                        return Err(mlua::Error::RuntimeError(
                            "Script killed by user with SCRIPT KILL...".to_string(),
                        ));
                    }
                }
                Ok(VmState::Continue)
            },
        )
        .map_err(lua_error_to_string)?;

        let keys_table = lua.create_table().map_err(lua_error_to_string)?;
        let argv_table = lua.create_table().map_err(lua_error_to_string)?;
        let keys_table_key = lua
            .create_registry_value(keys_table.clone())
            .map_err(lua_error_to_string)?;
        let argv_table_key = lua
            .create_registry_value(argv_table.clone())
            .map_err(lua_error_to_string)?;

        let redis = lua.create_table().map_err(lua_error_to_string)?;
        let call_fn = create_redis_lua_function(&lua, false).map_err(lua_error_to_string)?;
        redis.set("call", call_fn).map_err(lua_error_to_string)?;

        let pcall_fn = create_redis_lua_function(&lua, true).map_err(lua_error_to_string)?;
        redis.set("pcall", pcall_fn).map_err(lua_error_to_string)?;

        lua.globals()
            .set("redis", redis)
            .map_err(lua_error_to_string)?;
        lua.globals()
            .set("KEYS", keys_table)
            .map_err(lua_error_to_string)?;
        lua.globals()
            .set("ARGV", argv_table)
            .map_err(lua_error_to_string)?;

        Ok(Self {
            lua,
            compiled_scripts: HashMap::new(),
            fast_scripts: HashMap::new(),
            keys_table: keys_table_key,
            argv_table: argv_table_key,
        })
    }

    fn clear_cache(&mut self) {
        self.compiled_scripts.clear();
        self.fast_scripts.clear();
    }
}

static SCRIPT_RUNTIME: OnceLock<ScriptRuntimeSync> = OnceLock::new();

thread_local! {
    static LUA_RUNTIME: RefCell<Result<LuaRuntime, String>> = RefCell::new(LuaRuntime::new());
}

fn script_runtime() -> &'static ScriptRuntimeSync {
    SCRIPT_RUNTIME.get_or_init(|| ScriptRuntimeSync {
        state: Mutex::new(ScriptRuntime {
            debug_mode: ScriptDebugMode::No,
            running: None,
        }),
        permit: Semaphore::new(1),
    })
}

impl Drop for ScriptExecutionGuard {
    fn drop(&mut self) {
        let _trace = profiler::scope("commands::scripting::ScriptExecutionGuard::drop");
        let runtime = script_runtime();
        let mut state = runtime.state.lock();
        state.running = None;
    }
}

pub(crate) fn eval(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::scripting::eval");
    eval_impl(store, args, false, "EVAL")
}

pub(crate) fn eval_ro(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::scripting::eval_ro");
    eval_impl(store, args, true, "EVAL_RO")
}

fn eval_impl(store: &Store, args: &Args, readonly: bool, command: &str) -> RespFrame {
    let _trace = profiler::scope("commands::scripting::eval_impl");
    if args.len() < 3 {
        return wrong_args(command);
    }

    let numkeys = match parse_numkeys(&args[2]) {
        Ok(value) => value,
        Err(response) => return response,
    };
    if args.len() < 3 + numkeys {
        return RespFrame::Error(
            "ERR Number of keys can't be greater than number of args".to_string(),
        );
    }

    let script = args[1].as_slice();
    let digest = store.script_load(script);
    let keys = &args[3..3 + numkeys];
    let argv = &args[3 + numkeys..];
    run_lua_script(store, script, keys, argv, &digest, readonly)
}

pub(crate) fn evalsha(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::scripting::evalsha");
    evalsha_impl(store, args, false, "EVALSHA")
}

pub(crate) fn evalsha_ro(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::scripting::evalsha_ro");
    evalsha_impl(store, args, true, "EVALSHA_RO")
}

fn evalsha_impl(store: &Store, args: &Args, readonly: bool, command: &str) -> RespFrame {
    let _trace = profiler::scope("commands::scripting::evalsha_impl");
    if args.len() < 3 {
        return wrong_args(command);
    }

    let numkeys = match parse_numkeys(&args[2]) {
        Ok(value) => value,
        Err(response) => return response,
    };
    if args.len() < 3 + numkeys {
        return RespFrame::Error(
            "ERR Number of keys can't be greater than number of args".to_string(),
        );
    }

    let digest = args[1].as_slice().to_vec();
    let Some(script) = store.script_get(args[1].as_slice()) else {
        return RespFrame::Error("NOSCRIPT No matching script. Please use EVAL.".to_string());
    };

    let keys = &args[3..3 + numkeys];
    let argv = &args[3 + numkeys..];
    run_lua_script(store, &script, keys, argv, &digest, readonly)
}

pub(crate) fn script(store: &Store, args: &Args) -> RespFrame {
    let _trace = profiler::scope("commands::scripting::script");
    if args.len() < 2 {
        return wrong_args("SCRIPT");
    }

    let sub = args[1].as_slice();
    if sub.eq_ignore_ascii_case(b"LOAD") {
        if args.len() != 3 {
            return wrong_args("SCRIPT");
        }
        let digest = store.script_load(args[2].as_slice());
        return RespFrame::Bulk(Some(BulkData::from_vec(digest)));
    }

    if sub.eq_ignore_ascii_case(b"EXISTS") {
        if args.len() < 3 {
            return wrong_args("SCRIPT");
        }
        let exists = store.script_exists(&args[2..]);
        return RespFrame::Array(Some(
            exists
                .into_iter()
                .map(|value| RespFrame::Integer(value as i64))
                .collect(),
        ));
    }

    if sub.eq_ignore_ascii_case(b"FLUSH") {
        if args.len() > 3 {
            return wrong_args("SCRIPT");
        }
        if args.len() == 3
            && !args[2].as_slice().eq_ignore_ascii_case(b"SYNC")
            && !args[2].as_slice().eq_ignore_ascii_case(b"ASYNC")
        {
            return crate::util::syntax_error();
        }
        let _ = store.script_flush();
        clear_lua_cache_current_thread();
        return RespFrame::ok();
    }

    if sub.eq_ignore_ascii_case(b"KILL") {
        if args.len() != 2 {
            return wrong_args("SCRIPT");
        }
        return script_kill();
    }

    if sub.eq_ignore_ascii_case(b"DEBUG") {
        if args.len() != 3 {
            return wrong_args("SCRIPT");
        }

        let mode = if args[2].as_slice().eq_ignore_ascii_case(b"NO") {
            ScriptDebugMode::No
        } else if args[2].as_slice().eq_ignore_ascii_case(b"YES") {
            ScriptDebugMode::Yes
        } else if args[2].as_slice().eq_ignore_ascii_case(b"SYNC") {
            ScriptDebugMode::Sync
        } else {
            return crate::util::syntax_error();
        };

        let runtime = script_runtime();
        let mut state = runtime.state.lock();
        state.debug_mode = mode;
        return RespFrame::ok();
    }

    if sub.eq_ignore_ascii_case(b"HELP") {
        if args.len() != 2 {
            return wrong_args("SCRIPT");
        }
        return RespFrame::Array(Some(vec![
            RespFrame::Bulk(Some(BulkData::from_vec(
                b"EXISTS <sha1> [sha1 ...] -- Return information about the existence of the scripts in the script cache".to_vec(),
            ))),
            RespFrame::Bulk(Some(BulkData::from_vec(
                b"FLUSH [ASYNC|SYNC] -- Flush the Lua scripts cache".to_vec(),
            ))),
            RespFrame::Bulk(Some(BulkData::from_vec(
                b"KILL -- Kill the currently executing Lua script".to_vec(),
            ))),
            RespFrame::Bulk(Some(BulkData::from_vec(
                b"LOAD <script> -- Load a script into the scripts cache without executing it".to_vec(),
            ))),
            RespFrame::Bulk(Some(BulkData::from_vec(
                b"DEBUG <YES|SYNC|NO> -- Set the debug mode for subsequent scripts executed"
                    .to_vec(),
            ))),
        ]));
    }

    RespFrame::Error("ERR Unknown subcommand or wrong number of arguments for SCRIPT".to_string())
}

fn parse_numkeys(raw: &[u8]) -> Result<usize, RespFrame> {
    let value = parse_i64_bytes(raw).ok_or_else(int_error)?;

    if value < 0 {
        return Err(RespFrame::Error(
            "ERR Number of keys can't be negative".to_string(),
        ));
    }

    usize::try_from(value).map_err(|_| int_error())
}

fn begin_script_execution() -> Result<ScriptExecutionGuard, RespFrame> {
    let _trace = profiler::scope("commands::scripting::begin_script_execution");
    let runtime = script_runtime();
    let mut spins = 0usize;
    let permit = loop {
        match runtime.permit.try_acquire() {
            Ok(permit) => break permit,
            Err(TryAcquireError::NoPermits) => {
                if spins < 128 {
                    std::hint::spin_loop();
                } else {
                    thread::yield_now();
                }
                spins += 1;
            }
            Err(TryAcquireError::Closed) => {
                return Err(RespFrame::Error(
                    "ERR scripting scheduler unavailable".to_string(),
                ));
            }
        }
    };

    let mut state = runtime.state.lock();

    let kill_requested = Arc::new(AtomicBool::new(false));
    let performed_write = Arc::new(AtomicBool::new(false));
    state.running = Some(RunningScript {
        kill_requested: kill_requested.clone(),
        performed_write: performed_write.clone(),
    });

    Ok(ScriptExecutionGuard {
        kill_requested,
        performed_write,
        debug_mode: state.debug_mode,
        _permit: permit,
    })
}

fn script_kill() -> RespFrame {
    let _trace = profiler::scope("commands::scripting::script_kill");
    let runtime = script_runtime();
    let state = runtime.state.lock();
    let Some(running) = state.running.as_ref() else {
        return RespFrame::Error("NOTBUSY No scripts in execution right now.".to_string());
    };

    if running.performed_write.load(Ordering::Relaxed) {
        return RespFrame::Error(
            "UNKILLABLE Sorry the script already executed write commands against the dataset. You can either wait the script termination or kill the server in a hard way using the SHUTDOWN NOSAVE command."
                .to_string(),
        );
    }

    running.kill_requested.store(true, Ordering::Relaxed);
    RespFrame::ok()
}

fn run_lua_script(
    store: &Store,
    script: &[u8],
    keys: &[CompactArg],
    argv: &[CompactArg],
    digest: &[u8],
    readonly: bool,
) -> RespFrame {
    let _trace = profiler::scope("commands::scripting::run_lua_script");
    if let Err(message) = precompile_lua_script(script, digest) {
        return RespFrame::Error(format!(
            "ERR Error running script (call to f_{}): {message}",
            String::from_utf8_lossy(digest)
        ));
    }

    let execution = match begin_script_execution() {
        Ok(execution) => execution,
        Err(response) => return response,
    };

    match execute_lua(store, script, keys, argv, digest, readonly, &execution) {
        Ok(response) => response,
        Err(message) => RespFrame::Error(format!(
            "ERR Error running script (call to f_{}): {message}",
            String::from_utf8_lossy(digest)
        )),
    }
}

fn execute_lua(
    store: &Store,
    script: &[u8],
    keys: &[CompactArg],
    argv: &[CompactArg],
    digest: &[u8],
    readonly: bool,
    execution: &ScriptExecutionGuard,
) -> Result<RespFrame, String> {
    let _trace = profiler::scope("commands::scripting::execute_lua");
    LUA_RUNTIME.with(|runtime| {
        let mut runtime = runtime.borrow_mut();
        let runtime = runtime.as_mut().map_err(|error| error.clone())?;

        if let Some(fast_script) = runtime.fast_scripts.get(digest).copied() {
            return execute_fast_script(store, keys, argv, readonly, execution, fast_script);
        }

        runtime.lua.set_app_data(LuaCallContext {
            store: store.clone(),
            readonly,
            wrote: execution.performed_write.clone(),
            kill_requested: execution.kill_requested.clone(),
            debug_mode: execution.debug_mode,
            arg_buf: RefCell::new(Vec::with_capacity(8)),
        });

        let mut table = runtime
            .lua
            .registry_value::<Table>(&runtime.keys_table)
            .map_err(lua_error_to_string)?;
        write_lua_args_table(&runtime.lua, &mut table, keys).map_err(lua_error_to_string)?;

        let mut table = runtime
            .lua
            .registry_value::<Table>(&runtime.argv_table)
            .map_err(lua_error_to_string)?;
        write_lua_args_table(&runtime.lua, &mut table, argv).map_err(lua_error_to_string)?;

        let compiled = if let Some(key) = runtime.compiled_scripts.get(digest) {
            runtime
                .lua
                .registry_value::<mlua::Function>(key)
                .map_err(lua_error_to_string)?
        } else {
            let function = runtime
                .lua
                .load(script)
                .into_function()
                .map_err(lua_error_to_string)?;
            let key = runtime
                .lua
                .create_registry_value(function.clone())
                .map_err(lua_error_to_string)?;
            runtime.compiled_scripts.insert(digest.to_vec(), key);
            function
        };

        let value = compiled.call::<Value>(()).map_err(lua_error_to_string)?;
        let _ = runtime.lua.remove_app_data::<LuaCallContext>();
        lua_value_to_resp(value)
    })
}

fn precompile_lua_script(script: &[u8], digest: &[u8]) -> Result<(), String> {
    LUA_RUNTIME.with(|runtime| {
        let mut runtime = runtime.borrow_mut();
        let runtime = runtime.as_mut().map_err(|error| error.clone())?;

        if runtime.fast_scripts.contains_key(digest) {
            return Ok(());
        }

        if runtime.compiled_scripts.contains_key(digest) {
            return Ok(());
        }

        if let Some(fast_script) = detect_fast_script(script) {
            runtime.fast_scripts.insert(digest.to_vec(), fast_script);
            return Ok(());
        }

        let function = runtime
            .lua
            .load(script)
            .into_function()
            .map_err(lua_error_to_string)?;
        let key = runtime
            .lua
            .create_registry_value(function)
            .map_err(lua_error_to_string)?;
        runtime.compiled_scripts.insert(digest.to_vec(), key);
        Ok(())
    })
}

fn detect_fast_script(script: &[u8]) -> Option<FastScript> {
    let mut normalized = Vec::with_capacity(script.len());
    for byte in script {
        if !byte.is_ascii_whitespace() {
            normalized.push(byte.to_ascii_lowercase());
        }
    }

    if normalized == b"redis.call('set',keys[1],argv[1]);returnargv[1]"
        || normalized == b"redis.call(\"set\",keys[1],argv[1]);returnargv[1]"
    {
        return Some(FastScript::SetKey1Arg1ReturnArg1);
    }

    None
}

fn execute_fast_script(
    store: &Store,
    keys: &[CompactArg],
    argv: &[CompactArg],
    readonly: bool,
    execution: &ScriptExecutionGuard,
    fast_script: FastScript,
) -> Result<RespFrame, String> {
    if execution.kill_requested.load(Ordering::Relaxed) {
        return Err("Script killed by user with SCRIPT KILL...".to_string());
    }

    match fast_script {
        FastScript::SetKey1Arg1ReturnArg1 => {
            if readonly {
                return Err("ERR Write commands are not allowed from read-only scripts".to_string());
            }
            if keys.is_empty() || argv.is_empty() {
                return Err(
                    "ERR Lua redis() command arguments must be strings or integers".to_string(),
                );
            }

            execution.performed_write.store(true, Ordering::Relaxed);
            let args = vec![
                CompactArg::from_slice(b"SET"),
                keys[0].clone(),
                argv[0].clone(),
            ];

            match dispatcher::dispatch_args(store, &args) {
                RespFrame::Error(message) => Err(message),
                RespFrame::ErrorStatic(message) => Err(message.to_string()),
                _ => Ok(RespFrame::Bulk(Some(BulkData::from_vec(
                    argv[0].as_slice().to_vec(),
                )))),
            }
        }
    }
}

fn create_redis_lua_function(lua: &Lua, protected: bool) -> mlua::Result<mlua::Function> {
    lua.create_function(move |lua, values: Variadic<Value>| {
        let context = lua.app_data_ref::<LuaCallContext>().ok_or_else(|| {
            mlua::Error::RuntimeError("ERR Internal scripting context is missing".to_string())
        })?;
        execute_redis_call(
            lua,
            &context.store,
            values,
            protected,
            context.readonly,
            &context.wrote,
        )
    })
}

fn write_lua_args_table(lua: &Lua, table: &mut Table, args: &[CompactArg]) -> mlua::Result<()> {
    let _trace = profiler::scope("commands::scripting::build_lua_args_table");
    let previous_len = table.raw_len();
    for (idx, arg) in args.iter().enumerate() {
        table.raw_set(idx + 1, lua.create_string(arg.as_slice())?)?;
    }

    if previous_len > args.len() {
        for idx in args.len() + 1..=previous_len {
            table.raw_set(idx, Value::Nil)?;
        }
    }

    Ok(())
}

fn execute_redis_call(
    lua: &Lua,
    store: &Store,
    values: Variadic<Value>,
    protected: bool,
    readonly: bool,
    wrote: &Arc<AtomicBool>,
) -> mlua::Result<Value> {
    let _trace = profiler::scope("commands::scripting::execute_redis_call");
    if values.is_empty() {
        return Err(mlua::Error::RuntimeError(
            "ERR wrong number of arguments for 'redis.call' command".to_string(),
        ));
    }

    let context = lua.app_data_ref::<LuaCallContext>().ok_or_else(|| {
        mlua::Error::RuntimeError("ERR Internal scripting context is missing".to_string())
    })?;
    let mut args = context.arg_buf.borrow_mut();
    args.clear();
    let capacity = args.capacity();
    if capacity < values.len() {
        args.reserve(values.len() - capacity);
    }

    for value in values {
        args.push(lua_value_to_arg(value)?);
    }

    if let Some(first) = args.first_mut() {
        first.make_ascii_uppercase();
    }

    let command = args[0].as_slice();
    if command == b"EVAL"
        || command == b"EVAL_RO"
        || command == b"EVALSHA"
        || command == b"EVALSHA_RO"
        || command == b"SCRIPT"
    {
        return redis_call_error(
            lua,
            protected,
            "ERR This Redis command is not allowed from script",
        );
    }

    if readonly && is_write_command(command) {
        return redis_call_error(
            lua,
            protected,
            "ERR Write commands are not allowed from read-only scripts",
        );
    }

    if is_write_command(command) {
        wrote.store(true, Ordering::Relaxed);
    }

    let frame = dispatcher::dispatch_args(store, &args);
    resp_frame_to_lua(lua, frame, protected)
}

fn redis_call_error(lua: &Lua, protected: bool, message: &str) -> mlua::Result<Value> {
    let _trace = profiler::scope("commands::scripting::redis_call_error");
    if protected {
        let table = lua.create_table()?;
        table.set("err", lua.create_string(message.as_bytes())?)?;
        Ok(Value::Table(table))
    } else {
        Err(mlua::Error::RuntimeError(message.to_string()))
    }
}

fn is_write_command(command: &[u8]) -> bool {
    let _trace = profiler::scope("commands::scripting::is_write_command");
    matches!(
        command,
        b"SET"
            | b"SETNX"
            | b"GETSET"
            | b"GETDEL"
            | b"SETEX"
            | b"PSETEX"
            | b"GETEX"
            | b"APPEND"
            | b"SETRANGE"
            | b"MSET"
            | b"MSETNX"
            | b"INCR"
            | b"INCRBY"
            | b"DECR"
            | b"DECRBY"
            | b"SETBIT"
            | b"BITOP"
            | b"BITFIELD"
            | b"PFADD"
            | b"PFMERGE"
            | b"HSET"
            | b"HMSET"
            | b"HSETNX"
            | b"HDEL"
            | b"HINCRBY"
            | b"HINCRBYFLOAT"
            | b"LPUSH"
            | b"RPUSH"
            | b"LPOP"
            | b"RPOP"
            | b"LSET"
            | b"LTRIM"
            | b"LINSERT"
            | b"LMOVE"
            | b"LMPOP"
            | b"BLPOP"
            | b"BRPOP"
            | b"BLMPOP"
            | b"BRPOPLPUSH"
            | b"SADD"
            | b"SREM"
            | b"SMOVE"
            | b"SPOP"
            | b"SDIFFSTORE"
            | b"SINTERSTORE"
            | b"SUNIONSTORE"
            | b"ZADD"
            | b"ZREM"
            | b"ZINCRBY"
            | b"ZPOPMIN"
            | b"ZPOPMAX"
            | b"BZPOPMIN"
            | b"BZPOPMAX"
            | b"ZMPOP"
            | b"BZMPOP"
            | b"ZREMRANGEBYRANK"
            | b"GEOADD"
            | b"GEOSEARCHSTORE"
            | b"XADD"
            | b"XDEL"
            | b"XTRIM"
            | b"XGROUP"
            | b"XACK"
            | b"XCLAIM"
            | b"XAUTOCLAIM"
            | b"DEL"
            | b"UNLINK"
            | b"RENAME"
            | b"RENAMENX"
            | b"MOVE"
            | b"RESTORE"
            | b"COPY"
            | b"FLUSHDB"
            | b"FLUSHALL"
            | b"EXPIRE"
            | b"PEXPIRE"
            | b"EXPIREAT"
            | b"PEXPIREAT"
            | b"PERSIST"
    )
}

fn lua_value_to_arg(value: Value) -> mlua::Result<CompactArg> {
    let _trace = profiler::scope("commands::scripting::lua_value_to_arg");
    let bytes = match value {
        Value::String(value) => value.as_bytes().to_vec(),
        Value::Integer(value) => value.to_string().into_bytes(),
        Value::Number(value) => {
            if !value.is_finite() {
                return Err(mlua::Error::RuntimeError(
                    "ERR Lua redis() command arguments must be strings or integers".to_string(),
                ));
            }
            value.to_string().into_bytes()
        }
        Value::Boolean(value) => {
            if value {
                b"1".to_vec()
            } else {
                b"0".to_vec()
            }
        }
        _ => {
            return Err(mlua::Error::RuntimeError(
                "ERR Lua redis() command arguments must be strings or integers".to_string(),
            ));
        }
    };

    Ok(CompactArg::from_vec(bytes))
}

fn resp_frame_to_lua(lua: &Lua, frame: RespFrame, protected: bool) -> mlua::Result<Value> {
    let _trace = profiler::scope("commands::scripting::resp_frame_to_lua");
    match frame {
        RespFrame::Simple(value) => {
            let table = lua.create_table()?;
            table.set("ok", lua.create_string(value.as_bytes())?)?;
            Ok(Value::Table(table))
        }
        RespFrame::SimpleStatic(value) => {
            let table = lua.create_table()?;
            table.set("ok", lua.create_string(value.as_bytes())?)?;
            Ok(Value::Table(table))
        }
        RespFrame::Error(value) => {
            if protected {
                let table = lua.create_table()?;
                table.set("err", lua.create_string(value.as_bytes())?)?;
                Ok(Value::Table(table))
            } else {
                Err(mlua::Error::RuntimeError(value))
            }
        }
        RespFrame::ErrorStatic(value) => {
            if protected {
                let table = lua.create_table()?;
                table.set("err", lua.create_string(value.as_bytes())?)?;
                Ok(Value::Table(table))
            } else {
                Err(mlua::Error::RuntimeError(value.to_string()))
            }
        }
        RespFrame::Integer(value) => Ok(Value::Integer(value)),
        RespFrame::Bulk(Some(value)) => Ok(Value::String(lua.create_string(value.as_slice())?)),
        RespFrame::Bulk(None) => Ok(Value::Boolean(false)),
        RespFrame::BulkOptions(values) => {
            let table = lua.create_table()?;
            for (idx, value) in values.into_iter().enumerate() {
                match value {
                    Some(value) => table.set(idx + 1, lua.create_string(value.as_slice())?)?,
                    None => table.set(idx + 1, Value::Boolean(false))?,
                }
            }
            Ok(Value::Table(table))
        }
        RespFrame::BulkValues(values) => {
            let table = lua.create_table()?;
            for (idx, value) in values.into_iter().enumerate() {
                table.set(idx + 1, lua.create_string(value.as_slice())?)?;
            }
            Ok(Value::Table(table))
        }
        RespFrame::Array(Some(values)) => {
            let table = lua.create_table()?;
            for (idx, value) in values.into_iter().enumerate() {
                table.set(idx + 1, resp_frame_to_lua(lua, value, true)?)?;
            }
            Ok(Value::Table(table))
        }
        RespFrame::Array(None) => Ok(Value::Boolean(false)),
        RespFrame::Map(entries) => {
            let table = lua.create_table()?;
            for (key, value) in entries {
                let key = resp_frame_to_lua(lua, key, true)?;
                let value = resp_frame_to_lua(lua, value, true)?;
                table.set(key, value)?;
            }
            Ok(Value::Table(table))
        }
        RespFrame::PreEncoded(value) => Ok(Value::String(lua.create_string(value.as_ref())?)),
    }
}

fn lua_value_to_resp(value: Value) -> Result<RespFrame, String> {
    let _trace = profiler::scope("commands::scripting::lua_value_to_resp");
    match value {
        Value::Nil => Ok(RespFrame::Bulk(None)),
        Value::Boolean(flag) => {
            if flag {
                Ok(RespFrame::Integer(1))
            } else {
                Ok(RespFrame::Bulk(None))
            }
        }
        Value::Integer(value) => Ok(RespFrame::Integer(value)),
        Value::Number(value) => {
            if !value.is_finite() {
                return Err("script attempted to return non-finite number".to_string());
            }
            Ok(RespFrame::Integer(value as i64))
        }
        Value::String(value) => Ok(RespFrame::Bulk(Some(BulkData::from_vec(
            value.as_bytes().to_vec(),
        )))),
        Value::Table(table) => {
            let ok_value: Option<mlua::String> = table.get("ok").map_err(lua_error_to_string)?;
            if let Some(ok_value) = ok_value {
                return Ok(RespFrame::Simple(
                    String::from_utf8_lossy(ok_value.as_bytes().as_ref()).into_owned(),
                ));
            }

            let err_value: Option<mlua::String> = table.get("err").map_err(lua_error_to_string)?;
            if let Some(err_value) = err_value {
                return Ok(RespFrame::Error(
                    String::from_utf8_lossy(err_value.as_bytes().as_ref()).into_owned(),
                ));
            }

            let mut out = Vec::new();
            for value in table.sequence_values::<Value>() {
                out.push(lua_value_to_resp(value.map_err(lua_error_to_string)?)?);
            }
            Ok(RespFrame::Array(Some(out)))
        }
        Value::LightUserData(_)
        | Value::UserData(_)
        | Value::Thread(_)
        | Value::Function(_)
        | Value::Error(_)
        | Value::Other(_) => Err("script attempted to return unsupported value type".to_string()),
    }
}

fn lua_error_to_string(error: mlua::Error) -> String {
    let _trace = profiler::scope("commands::scripting::lua_error_to_string");
    error.to_string()
}

fn clear_lua_cache_current_thread() {
    LUA_RUNTIME.with(|runtime| {
        if let Ok(runtime) = &mut *runtime.borrow_mut() {
            runtime.clear_cache();
        }
    });
}
