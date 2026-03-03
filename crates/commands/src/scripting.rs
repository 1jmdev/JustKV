use crate::dispatcher;
use crate::util::{Args, int_error, wrong_args};
use engine::store::Store;
use engine::value::{CompactArg, CompactBytes};
use mlua::{HookTriggers, Lua, Table, Value, Variadic, VmState};
use parking_lot::Mutex;
use protocol::types::{BulkData, RespFrame};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};

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

struct ScriptExecutionGuard {
    kill_requested: Arc<AtomicBool>,
    performed_write: Arc<AtomicBool>,
    debug_mode: ScriptDebugMode,
}

static SCRIPT_RUNTIME: OnceLock<Mutex<ScriptRuntime>> = OnceLock::new();

fn script_runtime() -> &'static Mutex<ScriptRuntime> {
    SCRIPT_RUNTIME.get_or_init(|| {
        Mutex::new(ScriptRuntime {
            debug_mode: ScriptDebugMode::No,
            running: None,
        })
    })
}

impl Drop for ScriptExecutionGuard {
    fn drop(&mut self) {
        let _trace = profiler::scope("commands::scripting::ScriptExecutionGuard::drop");
        let mut runtime = script_runtime().lock();
        runtime.running = None;
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
            return RespFrame::Error("ERR syntax error".to_string());
        }
        let _ = store.script_flush();
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
            return RespFrame::Error("ERR syntax error".to_string());
        };

        let mut runtime = script_runtime().lock();
        runtime.debug_mode = mode;
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
    let _trace = profiler::scope("commands::scripting::parse_numkeys");
    let value = std::str::from_utf8(raw)
        .ok()
        .and_then(|raw| raw.parse::<i64>().ok())
        .ok_or_else(int_error)?;

    if value < 0 {
        return Err(RespFrame::Error(
            "ERR Number of keys can't be negative".to_string(),
        ));
    }

    usize::try_from(value).map_err(|_| int_error())
}

fn begin_script_execution() -> Result<ScriptExecutionGuard, RespFrame> {
    let _trace = profiler::scope("commands::scripting::begin_script_execution");
    let mut runtime = script_runtime().lock();
    if runtime.running.is_some() {
        return Err(RespFrame::Error(
            "BUSY Redis is busy running a script. You can only call SCRIPT KILL or SHUTDOWN NOSAVE."
                .to_string(),
        ));
    }

    let kill_requested = Arc::new(AtomicBool::new(false));
    let performed_write = Arc::new(AtomicBool::new(false));
    runtime.running = Some(RunningScript {
        kill_requested: kill_requested.clone(),
        performed_write: performed_write.clone(),
    });

    Ok(ScriptExecutionGuard {
        kill_requested,
        performed_write,
        debug_mode: runtime.debug_mode,
    })
}

fn script_kill() -> RespFrame {
    let _trace = profiler::scope("commands::scripting::script_kill");
    let runtime = script_runtime().lock();
    let Some(running) = runtime.running.as_ref() else {
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
    let execution = match begin_script_execution() {
        Ok(execution) => execution,
        Err(response) => return response,
    };

    match execute_lua(store, script, keys, argv, readonly, &execution) {
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
    readonly: bool,
    execution: &ScriptExecutionGuard,
) -> Result<RespFrame, String> {
    let _trace = profiler::scope("commands::scripting::execute_lua");
    let lua = Lua::new();

    let kill_requested = execution.kill_requested.clone();
    if execution.debug_mode != ScriptDebugMode::Sync {
        lua.set_hook(
            HookTriggers {
                every_nth_instruction: Some(1000),
                ..HookTriggers::default()
            },
            move |_lua, _debug| {
                if kill_requested.load(Ordering::Relaxed) {
                    Err(mlua::Error::RuntimeError(
                        "Script killed by user with SCRIPT KILL...".to_string(),
                    ))
                } else {
                    Ok(VmState::Continue)
                }
            },
        )
        .map_err(lua_error_to_string)?;
    }

    let globals = lua.globals();
    let keys_table = build_lua_args_table(&lua, keys).map_err(lua_error_to_string)?;
    let argv_table = build_lua_args_table(&lua, argv).map_err(lua_error_to_string)?;
    globals
        .set("KEYS", keys_table)
        .map_err(lua_error_to_string)?;
    globals
        .set("ARGV", argv_table)
        .map_err(lua_error_to_string)?;

    let redis = lua.create_table().map_err(lua_error_to_string)?;

    let call_store = store.clone();
    let call_wrote = execution.performed_write.clone();
    let call_fn = lua
        .create_function(move |lua, values: Variadic<Value>| {
            execute_redis_call(lua, &call_store, values, false, readonly, &call_wrote)
        })
        .map_err(lua_error_to_string)?;
    redis.set("call", call_fn).map_err(lua_error_to_string)?;

    let pcall_store = store.clone();
    let pcall_wrote = execution.performed_write.clone();
    let pcall_fn = lua
        .create_function(move |lua, values: Variadic<Value>| {
            execute_redis_call(lua, &pcall_store, values, true, readonly, &pcall_wrote)
        })
        .map_err(lua_error_to_string)?;
    redis.set("pcall", pcall_fn).map_err(lua_error_to_string)?;

    globals.set("redis", redis).map_err(lua_error_to_string)?;

    let value = lua
        .load(script)
        .eval::<Value>()
        .map_err(lua_error_to_string)?;

    lua_value_to_resp(value)
}

fn build_lua_args_table(lua: &Lua, args: &[CompactArg]) -> mlua::Result<Table> {
    let _trace = profiler::scope("commands::scripting::build_lua_args_table");
    let table = lua.create_table()?;
    for (idx, arg) in args.iter().enumerate() {
        table.set(idx + 1, lua.create_string(arg.as_slice())?)?;
    }
    Ok(table)
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

    let mut args = Vec::with_capacity(values.len());
    for value in values {
        args.push(lua_value_to_arg(value)?);
    }

    if let Some(first) = args.first_mut() {
        match first {
            CompactBytes::Inline { len, data } => data[..*len as usize].make_ascii_uppercase(),
            CompactBytes::Heap(value) => value.make_ascii_uppercase(),
        }
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
