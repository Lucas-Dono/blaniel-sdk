use npc_types::{ActionPlan, AutonomousAction};
use once_cell::sync::Lazy;
use regex::Regex;
use tracing::debug;

static BRACKET_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\[([^\]]+)\]").expect("Invalid bracket regex")
});

static COORDINATE_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^(-?\d+\.?\d*),\s*(-?\d+\.?\d*),\s*(-?\d+\.?\d*)(?:,\s*([^,]+))?(?:,\s*(-?\d+\.?\d*))?$")
        .expect("Invalid coordinate regex")
});

#[derive(Debug, Clone, PartialEq)]
pub enum MovementCommand {
    Navigate {
        target: String,
        speed: Option<f64>,
    },
    NavigateCoords {
        x: f64,
        y: f64,
        z: f64,
        world: Option<String>,
        speed: Option<f64>,
    },
    NavigateRelative {
        dx: f64,
        dy: f64,
        dz: f64,
        speed: Option<f64>,
    },
    ActionSequence {
        actions: Vec<AutonomousAction>,
        loop_plan: bool,
        interrupt_on_chat: bool,
    },
}

pub struct MovementCommandParser;

impl MovementCommandParser {
    pub fn new() -> Self {
        Self
    }

    pub fn parse_response(response: &str) -> Vec<MovementCommand> {
        let mut commands = Vec::new();

        if let Some(cmd) = Self::parse_bracket_command(response) {
            commands.push(cmd);
        }

        if commands.is_empty() {
            if let Some(plan) = Self::parse_json_plan(response) {
                commands.push(plan);
            }
        }

        if commands.is_empty() {
            if let Some(cmd) = Self::parse_natural_language(response) {
                commands.push(cmd);
            }
        }

        commands
    }

    fn parse_bracket_command(response: &str) -> Option<MovementCommand> {
        let re = regex_inner(response)?;

        if re.starts_with("navigate:") || re.starts_with("location:") {
            let inner = re.split(':').nth(1).unwrap_or("").trim();

            if inner.contains(',') {
                return Self::parse_coordinate_command(inner);
            }

            let parts: Vec<&str> = inner.split('|').collect();
            let target = parts[0].trim().to_string();
            let speed = parts.get(1).and_then(|s| s.trim().parse::<f64>().ok());

            Some(MovementCommand::Navigate { target, speed })
        } else if re.starts_with("coords:") || re.starts_with("coordinates:") {
            let inner = re.split(':').nth(1).unwrap_or("").trim();
            Self::parse_coordinate_command(inner)
        } else if re.starts_with("move:") {
            let inner = re.split(':').nth(1).unwrap_or("").trim();
            let parts: Vec<&str> = inner.split(',').collect();
            if parts.len() >= 3 {
                if let (Ok(dx), Ok(dy), Ok(dz)) = (
                    parts[0].trim().parse::<f64>(),
                    parts[1].trim().parse::<f64>(),
                    parts[2].trim().parse::<f64>(),
                ) {
                    let speed = parts.get(3).and_then(|s| s.trim().parse::<f64>().ok());
                    return Some(MovementCommand::NavigateRelative { dx, dy, dz, speed });
                }
            }
            None
        } else {
            None
        }
    }

    fn parse_coordinate_command(inner: &str) -> Option<MovementCommand> {
        let parts: Vec<&str> = inner.split(',').collect();
        if parts.len() >= 3 {
            if let (Ok(x), Ok(y), Ok(z)) = (
                parts[0].trim().parse::<f64>(),
                parts[1].trim().parse::<f64>(),
                parts[2].trim().parse::<f64>(),
            ) {
                let world = parts.get(3).map(|s| s.trim().to_string());
                let speed = parts.get(4).and_then(|s| s.trim().parse::<f64>().ok());
                return Some(MovementCommand::NavigateCoords {
                    x,
                    y,
                    z,
                    world,
                    speed,
                });
            }
        }
        None
    }

    fn parse_json_plan(response: &str) -> Option<MovementCommand> {
        let start = response.find("\"actions\"")?;
        let json_start = response[..start].rfind('{')?;
        let json_str = &response[json_start..];

        let mut depth = 0;
        let mut end = json_str.len();
        for (i, c) in json_str.char_indices() {
            match c {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        end = i + 1;
                        break;
                    }
                }
                _ => {}
            }
        }

        let plan_json = &json_str[..end];
        let plan: ActionPlan = serde_json::from_str(plan_json).ok()?;

        Some(MovementCommand::ActionSequence {
            loop_plan: plan.loop_plan,
            interrupt_on_chat: plan.interrupt_on_chat,
            actions: plan.actions,
        })
    }

    fn parse_natural_language(response: &str) -> Option<MovementCommand> {
        let lower = response.to_lowercase();

        let patterns = [
            ("go to the ", 10),
            ("walk to the ", 12),
            ("head to the ", 12),
            ("move to the ", 12),
            ("ir a la ", 8),
            ("ir al ", 6),
            ("caminar hasta ", 14),
            ("moverse a ", 10),
            ("ir hacia ", 9),
        ];

        for (pattern, skip) in &patterns {
            if let Some(pos) = lower.find(pattern) {
                let after = &lower[pos + skip..];
                let target = after
                    .split_whitespace()
                    .take(3)
                    .collect::<Vec<_>>()
                    .join(" ")
                    .trim_end_matches('.')
                    .trim_end_matches(',')
                    .trim_end_matches('!')
                    .to_string();

                if !target.is_empty() {
                    debug!(
                        "Natural language movement detected: '{}' -> target '{}'",
                        &lower[pos..pos + skip + 10.min(after.len())],
                        target
                    );
                    return Some(MovementCommand::Navigate {
                        target,
                        speed: None,
                    });
                }
            }
        }

        None
    }

    pub fn strip_commands(response: &str) -> String {
        static COMMAND_PREFIXES: &[&str] = &[
            "navigate:",
            "location:",
            "coords:",
            "coordinates:",
            "move:",
        ];

        let mut result = String::with_capacity(response.len());
        let mut last_end = 0;

        for captures in BRACKET_REGEX.captures_iter(response) {
            let full_match = captures.get(0).unwrap();
            let bracket_content = captures.get(1).unwrap().as_str();

            let is_command = COMMAND_PREFIXES
                .iter()
                .any(|prefix| bracket_content.starts_with(prefix));

            if is_command {
                let before = &response[last_end..full_match.start()];
                result.push_str(before.trim_end());
                if !result.is_empty() && !result.ends_with(' ') {
                    let after_start = full_match.end();
                    if after_start < response.len() && !response[after_start..].starts_with(' ') {
                        result.push(' ');
                    }
                }
                last_end = full_match.end();
            }
        }

        result.push_str(&response[last_end..]);
        result.trim().to_string()
    }
}

fn regex_inner(response: &str) -> Option<String> {
    BRACKET_REGEX
        .captures(response)
        .and_then(|cap| cap.get(1))
        .map(|m| m.as_str().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_word_command() {
        let cmd = MovementCommandParser::parse_bracket_command("Voy a descansar [navigate:bed]");
        assert!(cmd.is_some());
        let cmd = cmd.unwrap();
        assert_eq!(
            cmd,
            MovementCommand::Navigate {
                target: "bed".to_string(),
                speed: None,
            }
        );
    }

    #[test]
    fn test_parse_location_command() {
        let cmd = MovementCommandParser::parse_bracket_command("[location:cama]");
        assert!(cmd.is_some());
        let cmd = cmd.unwrap();
        assert_eq!(
            cmd,
            MovementCommand::Navigate {
                target: "cama".to_string(),
                speed: None,
            }
        );
    }

    #[test]
    fn test_parse_coordinate_command() {
        let cmd = MovementCommandParser::parse_bracket_command("[coords:10.5,0.0,20.3,house]");
        assert!(cmd.is_some());
        let cmd = cmd.unwrap();
        match cmd {
            MovementCommand::NavigateCoords { x, y, z, world, .. } => {
                assert_eq!(x, 10.5);
                assert_eq!(y, 0.0);
                assert_eq!(z, 20.3);
                assert_eq!(world, Some("house".to_string()));
            }
            _ => panic!("Expected NavigateCoords"),
        }
    }

    #[test]
    fn test_parse_relative_command() {
        let cmd = MovementCommandParser::parse_bracket_command("[move:5,0,-3]");
        assert!(cmd.is_some());
        let cmd = cmd.unwrap();
        assert_eq!(
            cmd,
            MovementCommand::NavigateRelative {
                dx: 5.0,
                dy: 0.0,
                dz: -3.0,
                speed: None,
            }
        );
    }

    #[test]
    fn test_parse_natural_language_english() {
        let cmd = MovementCommandParser::parse_natural_language("I will go to the kitchen now");
        assert!(cmd.is_some());
        let cmd = cmd.unwrap();
        match cmd {
            MovementCommand::Navigate { target, .. } => {
                assert_eq!(target, "kitchen now");
            }
            _ => panic!("Expected Navigate"),
        }
    }

    #[test]
    fn test_parse_natural_language_spanish() {
        let cmd = MovementCommandParser::parse_natural_language("Voy a ir a la cama");
        assert!(cmd.is_some());
    }

    #[test]
    fn test_strip_commands() {
        let response = "Voy a descansar [navigate:bed] en un momento";
        let clean = MovementCommandParser::strip_commands(response);
        assert_eq!(clean, "Voy a descansar en un momento");
    }

    #[test]
    fn test_strip_commands_multiple() {
        let response = "Voy [navigate:bed] y luego [navigate:kitchen]";
        let clean = MovementCommandParser::strip_commands(response);
        assert_eq!(clean, "Voy y luego");
    }

    #[test]
    fn test_parse_with_speed() {
        let cmd = MovementCommandParser::parse_bracket_command("[navigate:bed|2.5]");
        assert!(cmd.is_some());
        let cmd = cmd.unwrap();
        assert_eq!(
            cmd,
            MovementCommand::Navigate {
                target: "bed".to_string(),
                speed: Some(2.5),
            }
        );
    }

    #[test]
    fn test_full_parse_response() {
        let commands =
            MovementCommandParser::parse_response("Me voy a dormir [navigate:cama] buenas noches");
        assert_eq!(commands.len(), 1);
    }
}
