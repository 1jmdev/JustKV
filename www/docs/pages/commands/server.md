# Connection & Server Commands

Commands for connection setup and basic server interaction.

## PING / ECHO

```
PING [message]
ECHO message
```

Connectivity and round-trip checks.

**Complexity:** O(1)

```bash
PING
PING "ready?"
ECHO "hello"
```

## AUTH

```
AUTH [username] password
```

Authenticate a client connection.

**Complexity:** O(1)

```bash
AUTH my-secret-password
AUTH default my-secret-password
```

## HELLO

```
HELLO [protover [AUTH username password] [SETNAME clientname]]
```

Perform protocol handshake and optional authentication.

**Complexity:** O(1)

```bash
HELLO 3
HELLO 3 AUTH default my-secret-password
```

## CLIENT

```
CLIENT <subcommand> [arguments ...]
```

Client management command namespace.

```bash
CLIENT ID
CLIENT LIST
CLIENT SETNAME api-worker-1
```

## SELECT

```
SELECT index
```

Switch active logical database for the current connection.

**Complexity:** O(1)

```bash
SELECT 0
SELECT 1
```

## COMMAND

```
COMMAND
```

Returns command table information. In the current dispatcher implementation this returns an empty array response.

```bash
COMMAND
```

## QUIT

```
QUIT
```

Close the client connection.

```bash
QUIT
```
