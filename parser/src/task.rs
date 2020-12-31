use pest::error::Error as PestError;
use pest::error::ErrorVariant;
use pest::iterators::Pairs;
use pest::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[grammar = "task.pest"]
struct TaskParser;

#[derive(Clone, Debug)]
pub struct Task {
    pub name: String,
    pub inline: Option<String>,
}

impl Task {
    pub fn get_script(&self) -> Option<&String> {
        self.inline.as_ref()
    }

    pub fn new(name: &str) -> Self {
        Task {
            name: name.to_string(),
            inline: None,
        }
    }

    fn parse(parser: &mut Pairs<Rule>) -> Result<Self, PestError<Rule>> {
        let mut task: Option<Self> = None;

        while let Some(parsed) = parser.next() {
            match parsed.as_rule() {
                Rule::identifier => {
                    task = Some(Task::new(parsed.as_str()));
                }
                Rule::script => {
                    let script = parse_str(&mut parsed.into_inner())?;

                    if let Some(ref mut task) = task {
                        task.inline = Some(script);
                    }
                }
                _ => {}
            }
        }

        if let Some(task) = task {
            return Ok(task);
        } else {
            return Err(PestError::new_from_pos(
                ErrorVariant::CustomError {
                    message: "Could not find a valid task definition".to_string(),
                },
                /* TODO: Find a better thing to report */
                pest::Position::from_start(""),
            ));
        }
    }

    pub fn from_str(buf: &str) -> Result<Self, PestError<Rule>> {
        let mut parser = TaskParser::parse(Rule::task, buf)?;
        while let Some(parsed) = parser.next() {
            match parsed.as_rule() {
                Rule::task => {
                    return Task::parse(&mut parsed.into_inner());
                }
                _ => {}
            }
        }

        Err(PestError::new_from_pos(
            ErrorVariant::CustomError {
                message: "Could not find a valid task definition".to_string(),
            },
            pest::Position::from_start(buf),
        ))
    }

    pub fn from_path(path: &PathBuf) -> Result<Self, PestError<Rule>> {
        use std::fs::File;
        use std::io::Read;

        match File::open(path) {
            Ok(mut file) => {
                let mut contents = String::new();

                if let Err(e) = file.read_to_string(&mut contents) {
                    return Err(PestError::new_from_pos(
                        ErrorVariant::CustomError {
                            message: format!("{}", e),
                        },
                        pest::Position::from_start(""),
                    ));
                } else {
                    return Self::from_str(&contents);
                }
            }
            Err(e) => {
                return Err(PestError::new_from_pos(
                    ErrorVariant::CustomError {
                        message: format!("{}", e),
                    },
                    pest::Position::from_start(""),
                ));
            }
        }
    }
}

/**
 * Parser utility function to fish out the _actual_ string value for something
 * that is looking like a string Rule
 */
fn parse_str(parser: &mut Pairs<Rule>) -> Result<String, PestError<Rule>> {
    while let Some(parsed) = parser.next() {
        match parsed.as_rule() {
            Rule::string => {
                return parse_str(&mut parsed.into_inner());
            }
            Rule::double_quoted => {
                return parse_str(&mut parsed.into_inner());
            }
            Rule::inner_double_str => {
                return Ok(parsed.as_str().to_string());
            }
            _ => {}
        }
    }
    return Err(PestError::new_from_pos(
        ErrorVariant::CustomError {
            message: "Could not parse out a string value".to_string(),
        },
        /* TODO: Find a better thing to report */
        pest::Position::from_start(""),
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_script_task() {
        let buf = r#"task Install {
                parameters {
                    package {
                        required = true
                        help = "Name of package to be installed"
                        type = string
                    }
                }
                script {
                    inline = "zypper in -y ${ZAP_PACKAGE}"
                }
            }"#;
        let _task = TaskParser::parse(Rule::task, buf).unwrap().next().unwrap();
    }

    #[test]
    fn parse_no_parameters() {
        let buf = r#"task PrintEnv {
                script {
                    inline = "env"
                }
            }"#;
        let _task = TaskParser::parse(Rule::task, buf).unwrap().next().unwrap();
    }

    #[test]
    fn parse_task_fn() {
        let buf = r#"task PrintEnv {
                script {
                    inline = "env"
                }
            }"#;
        let task = Task::from_str(buf).expect("Failed to parse the task");
        assert_eq!(task.name, "PrintEnv");
        assert_eq!(task.get_script().unwrap(), "env");
    }
}
