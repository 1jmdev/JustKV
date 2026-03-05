# Scripting Commands

Lua scripting commands execute server-side logic atomically.

## EVAL / EVAL_RO

```
EVAL script numkeys key [key ...] arg [arg ...]
EVAL_RO script numkeys key [key ...] arg [arg ...]
```

Execute a script directly from source text.

**Complexity:** Depends on script body

```bash
EVAL "return redis.call('GET', KEYS[1])" 1 user:1
EVAL_RO "return redis.call('TTL', KEYS[1])" 1 session:1
```

## EVALSHA / EVALSHA_RO

```
EVALSHA sha1 numkeys key [key ...] arg [arg ...]
EVALSHA_RO sha1 numkeys key [key ...] arg [arg ...]
```

Execute a previously loaded script by SHA1 digest.

```bash
EVALSHA 1b936e3fe509bcbc9cd0664897bbe8fd0cac101b 1 user:1
EVALSHA_RO 1b936e3fe509bcbc9cd0664897bbe8fd0cac101b 1 user:1
```

## SCRIPT

```
SCRIPT LOAD script
SCRIPT EXISTS sha1 [sha1 ...]
SCRIPT FLUSH [ASYNC | SYNC]
SCRIPT KILL
SCRIPT HELP
```

Manage the script cache and script lifecycle.

```bash
SCRIPT LOAD "return redis.call('PING')"
SCRIPT EXISTS 1b936e3fe509bcbc9cd0664897bbe8fd0cac101b
SCRIPT FLUSH
```
