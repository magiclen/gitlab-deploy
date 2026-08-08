#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) enum Command {
    Up,
    Stop,
    Down,
    Logs,
    DownAndUp,
}

impl Command {
    #[inline]
    pub(crate) fn parse_str<S: AsRef<str>>(s: S) -> Result<Self, ()> {
        let s = s.as_ref();

        let command = match s.to_ascii_lowercase().as_str() {
            "start" | "up" => Command::Up,
            "stop" => Command::Stop,
            "down" => Command::Down,
            "log" | "logs" => Command::Logs,
            "down_up" | "restart" => Command::DownAndUp,
            _ => return Err(()),
        };

        Ok(command)
    }

    #[inline]
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Up => "up",
            Self::Stop => "stop",
            Self::Down => "down",
            Self::Logs => "logs",
            Self::DownAndUp => "down_up",
        }
    }

    #[inline]
    pub(crate) fn get_command_str(&self) -> &'static str {
        match self {
            Self::Up | Self::DownAndUp => {
                "docker compose up -d --build && (timeout 10 docker compose logs -f || true)"
            },
            Self::Stop => "docker compose stop",
            Self::Down => "docker compose down",
            Self::Logs => "docker compose logs",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_every_alias() {
        assert_eq!(Command::Up, Command::parse_str("start").unwrap());
        assert_eq!(Command::Up, Command::parse_str("up").unwrap());
        assert_eq!(Command::Stop, Command::parse_str("stop").unwrap());
        assert_eq!(Command::Down, Command::parse_str("down").unwrap());
        assert_eq!(Command::Logs, Command::parse_str("log").unwrap());
        assert_eq!(Command::Logs, Command::parse_str("logs").unwrap());
        assert_eq!(Command::DownAndUp, Command::parse_str("down_up").unwrap());
        assert_eq!(Command::DownAndUp, Command::parse_str("restart").unwrap());
    }

    #[test]
    fn parse_is_case_insensitive() {
        assert_eq!(Command::DownAndUp, Command::parse_str("ReStArT").unwrap());
    }

    #[test]
    fn as_str_round_trips_back_to_the_same_command() {
        for command in
            [Command::Up, Command::Stop, Command::Down, Command::Logs, Command::DownAndUp]
        {
            assert_eq!(command, Command::parse_str(command.as_str()).unwrap());
        }
    }
}
