#[derive(Debug, Eq, PartialEq)]
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
                "docker-compose up -d --build && (timeout 10 docker-compose logs -f || true)"
            },
            Self::Stop => "docker-compose stop",
            Self::Down => "docker-compose down",
            Self::Logs => "docker-compose logs",
        }
    }
}
