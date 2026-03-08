use super::Args;

pub fn apply_uri(args: &mut Args, raw: &str) -> Result<(), String> {
    let (scheme, rest) = raw
        .split_once("://")
        .ok_or_else(|| format!("invalid URI {raw:?}"))?;

    if scheme != "valkey" && scheme != "redis" {
        return Err(format!("unsupported URI scheme {scheme:?}"));
    }

    let (authority, path) = match rest.split_once('/') {
        Some((authority, path)) => (authority, Some(path)),
        None => (rest, None),
    };

    let (auth_part, host_part) = match authority.rsplit_once('@') {
        Some((auth, host)) => (Some(auth), host),
        None => (None, authority),
    };

    if let Some(auth) = auth_part {
        let (user, password) = match auth.split_once(':') {
            Some((user, password)) => (Some(user), password),
            None => (None, auth),
        };

        if let Some(user) = user {
            args.user = Some(user.to_string());
        }
        args.password = Some(password.to_string());
    }

    let (host, port) = match host_part.rsplit_once(':') {
        Some((host, port)) if !port.is_empty() => {
            let port = port
                .parse::<u16>()
                .map_err(|err| format!("invalid port in URI: {err}"))?;
            (host, port)
        }
        _ => (host_part, 6379),
    };

    if !host.is_empty() {
        args.host = host.to_string();
    }
    args.port = port;

    if let Some(path) = path.filter(|value| !value.is_empty()) {
        args.dbnum = path
            .parse::<u32>()
            .map_err(|err| format!("invalid db number in URI: {err}"))?;
    }

    Ok(())
}
