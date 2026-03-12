# Security

BetterKV is fast, but security still starts with deployment discipline. Treat it like any other Redis-compatible data service: private network first, explicit auth, and least privilege.

## Security baseline

- bind only to loopback or private interfaces
- enable authentication before exposing the service to applications
- use ACLs for role separation
- prefer TLS for production traffic
- disable or restrict dangerous admin commands

## Network hardening

```ini title="betterkv.conf"
bind 127.0.0.1 10.0.0.5
protected-mode yes
```

```bash
ufw allow from 10.0.0.0/24 to any port 6379
ufw deny 6379
```

## Authentication

Simple password:

```ini title="betterkv.conf"
requirepass your_strong_password_here
```

ACL file:

```ini title="/etc/betterkv/users.acl"
user default off
user admin on >admin_password ~* &* +@all
user app on >app_password ~session:* ~cache:* +GET +SET +DEL +EXPIRE +TTL
```

## TLS

```ini title="betterkv.conf"
tls-port 6380
port 0
tls-cert-file /etc/betterkv/tls/server.crt
tls-key-file /etc/betterkv/tls/server.key
tls-ca-cert-file /etc/betterkv/tls/ca.crt
tls-auth-clients yes
```

## Restrict dangerous commands

```ini title="betterkv.conf"
rename-command FLUSHDB ""
rename-command FLUSHALL ""
rename-command CONFIG ""
rename-command DEBUG ""
```

## Security checklist

- [ ] private bind addresses only
- [ ] `protected-mode yes`
- [ ] strong password or ACLs
- [ ] TLS for production links
- [ ] dangerous commands restricted
- [ ] non-root service user
- [ ] firewall rules in front of the service

## Comparison note

Security should not be a differentiator by hand-wavy marketing. If you compare BetterKV with Redis and Valkey here, keep the message grounded: compatible operational model, modern deployment defaults, and Elastic License 2.0 for BetterKV.

## License

BetterKV is licensed under **Elastic License 2.0**.
