mod builtin;
mod template;

use crate::cli::Args;

pub use builtin::tests;
pub use template::{ArgTemplate, CommandTemplate};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BenchKind {
    PingInline,
    PingMbulk,
    Set,
    Get,
    Incr,
    Lpush,
    Rpush,
    Lpop,
    Rpop,
    Sadd,
    Hset,
    Spop,
    Zadd,
    ZpopMin,
    Lrange100,
    Lrange300,
    Lrange500,
    Lrange600,
    Mset,
    Custom,
}

#[derive(Clone, Copy, Debug)]
pub struct BenchSpec {
    pub key: &'static str,
    pub name: &'static str,
    pub kind: BenchKind,
}

#[derive(Clone, Debug)]
pub struct BenchRun {
    pub name: String,
    pub kind: BenchKind,
    pub clients: usize,
    pub requests: u64,
    pub data_size: usize,
    pub pipeline: usize,
    pub random_keyspace_len: Option<u64>,
    pub dbnum: u32,
    pub keep_alive: bool,
    pub key_prefix: String,
    pub seed: u64,
    pub command: Option<CommandTemplate>,
}

pub fn resolve_workload(
    args: &Args,
    stdin_last_arg: Option<Vec<u8>>,
) -> Result<Vec<BenchRun>, String> {
    if !args.command_args.is_empty() {
        return Ok(vec![BenchRun {
            name: args.command_args[0].to_ascii_uppercase(),
            kind: BenchKind::Custom,
            clients: args.clients,
            requests: args.requests,
            data_size: args.data_size,
            pipeline: args.pipeline,
            random_keyspace_len: args.random_keyspace_len,
            dbnum: args.dbnum,
            keep_alive: args.keep_alive_enabled(),
            key_prefix: "betterkv-benchmark".to_string(),
            seed: args.random_seed(),
            command: Some(template::build_custom_command(args, stdin_last_arg)?),
        }]);
    }

    let selected = if args.tests.is_empty() {
        tests().to_vec()
    } else {
        args.tests
            .iter()
            .map(|name| builtin::find_test(name).ok_or_else(|| builtin::unknown_test_error(name)))
            .collect::<Result<Vec<_>, _>>()?
    };

    Ok(selected
        .into_iter()
        .map(|spec| BenchRun {
            name: spec.name.to_string(),
            kind: spec.kind,
            clients: args.clients,
            requests: args.requests,
            data_size: args.data_size,
            pipeline: args.pipeline,
            random_keyspace_len: args.random_keyspace_len,
            dbnum: args.dbnum,
            keep_alive: args.keep_alive_enabled(),
            key_prefix: "betterkv-benchmark".to_string(),
            seed: args.random_seed(),
            command: None,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_args() -> Args {
        Args {
            host: "127.0.0.1".to_string(),
            port: 6379,
            socket: None,
            password: None,
            user: None,
            uri: None,
            clients: 20,
            requests: 1_000,
            data_size: 3,
            dbnum: 0,
            resp3: false,
            threads: Some(4),
            cluster: false,
            read_from_replicas: "no".to_string(),
            enable_tracking: false,
            keep_alive: 1,
            random_keyspace_len: Some(500),
            pipeline: 4,
            quiet: false,
            precision: 3,
            csv: false,
            loop_forever: false,
            tests: Vec::new(),
            idle_mode: false,
            read_last_arg_from_stdin: false,
            seed: Some(42),
            num_functions: 10,
            num_keys_in_fcall: 1,
            tls: false,
            sni: None,
            cacert: None,
            cacertdir: None,
            insecure: false,
            cert: None,
            key: None,
            tls_ciphers: None,
            tls_ciphersuites: None,
            strict: false,
            command_args: Vec::new(),
        }
    }

    #[test]
    fn resolves_default_tests_when_no_selection_is_given() {
        let runs = resolve_workload(&sample_args(), None).expect("resolve workload");
        assert_eq!(runs.len(), builtin::TESTS.len());
        assert_eq!(runs[0].name, "PING_INLINE");
    }

    #[test]
    fn resolves_custom_command() {
        let mut args = sample_args();
        args.command_args = vec![
            "lpush".to_string(),
            "mylist".to_string(),
            "__rand_int__".to_string(),
        ];

        let runs = resolve_workload(&args, None).expect("resolve workload");
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].name, "LPUSH");
        assert!(matches!(
            runs[0].command.as_ref().unwrap().parts[2],
            ArgTemplate::RandomInt
        ));
    }
}
