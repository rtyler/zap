use log::*;
use pest::error::Error as PestError;
use pest::error::ErrorVariant;
use pest::iterators::Pairs;
use pest::Parser;
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

#[derive(Parser)]
#[grammar = "task.pest"]
struct TaskParser;

/**
 * A Script represents something that can be executed oa a remote host.
 *
 * These come in two variants:
 *   - Inline string of shell commands to run
 *   - A script or binary file to transfer and execute
 */
#[derive(Clone, Debug)]
pub struct Script {
    /**
     * Inline scripts will have parameters rendered into then with handlebars syntax
     */
    pub inline: Option<String>,
    /**
     * File scripts will be executed with the parameters passed as command line
     * arguments, e.g. the "msg" parameter would be passed as:
     *      ./file --msg=value
     */
    pub file: Option<PathBuf>,
}

impl Script {
    fn new() -> Self {
        Self {
            inline: None,
            file: None,
        }
    }

    pub fn has_file(&self) -> bool {
        self.file.is_some()
    }

    /**
     * Return the script's contents as bytes
     *
     * This is useful for transferring the script to another host for execution
     *
     * If the `file` member is defined, that will be preferred, even if `inline` is also defined
     */
    pub fn as_bytes(&self, parameters: Option<&HashMap<String, String>>) -> Option<Vec<u8>> {
        use handlebars::Handlebars;

        if self.inline.is_some() && self.file.is_some() {
            warn!("Both inline and file structs are defined for this script, only file will be used!\n({})",
            self.inline.as_ref().unwrap());
        }

        if let Some(path) = &self.file {
            match File::open(path) {
                Ok(mut file) => {
                    let mut buf = vec![];

                    if let Ok(count) = file.read_to_end(&mut buf) {
                        debug!("Read {} bytes of {}", count, path.display());
                        return Some(buf);
                    } else {
                        error!("Failed to read the file {}", path.display());
                    }
                }
                Err(err) => {
                    error!("Failed to open the file at {},  {:?}", path.display(), err);
                }
            }
        }

        if let Some(inline) = &self.inline {
            // Early exit if there are no parameters to render
            if parameters.is_none() {
                return Some(inline.as_bytes().to_vec());
            }

            let parameters = parameters.unwrap();

            let mut hb = Handlebars::new();
            hb.register_escape_fn(handlebars::no_escape);
            match hb.render_template(inline, &parameters) {
                Ok(rendered) => {
                    return Some(rendered.as_bytes().to_vec());
                }
                Err(err) => {
                    error!("Failed to render command ({:?}): {}", err, inline);
                    return Some(inline.as_bytes().to_vec());
                }
            }
        }

        None
    }
}

#[derive(Clone, Debug)]
pub struct Task {
    pub name: String,
    pub script: Script,
}

impl Task {
    pub fn new(name: &str) -> Self {
        Task {
            name: name.to_string(),
            script: Script::new(),
        }
    }

    fn parse(parser: &mut Pairs<Rule>) -> Result<Self, PestError<Rule>> {
        let mut task: Option<Self> = None;
        let mut inline = None;
        let mut file = None;

        while let Some(parsed) = parser.next() {
            match parsed.as_rule() {
                Rule::identifier => {
                    task = Some(Task::new(parsed.as_str()));
                }
                Rule::script => {
                    for pair in parsed.into_inner() {
                        match pair.as_rule() {
                            Rule::script_inline => {
                                inline = Some(parse_str(&mut pair.into_inner())?);
                            }
                            Rule::script_file => {
                                let path = parse_str(&mut pair.into_inner())?;
                                file = Some(PathBuf::from(path));
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }

        if let Some(mut task) = task {
            task.script.inline = inline;
            task.script.file = file;

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
            Rule::triple_quoted => {
                return parse_str(&mut parsed.into_inner());
            }
            Rule::single_quoted => {
                return parse_str(&mut parsed.into_inner());
            }
            Rule::inner_single_str => {
                return Ok(parsed.as_str().to_string());
            }
            Rule::inner_triple_str => {
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
                        help = 'Name of package to be installed'
                        type = string
                    }
                }
                script {
                    inline = 'zypper in -y {{package}}'
                }
            }"#;
        let _task = TaskParser::parse(Rule::task, buf).unwrap().next().unwrap();
    }

    #[test]
    fn parse_no_parameters() {
        let buf = r#"task PrintEnv {
                script {
                    inline = 'env'
                }
            }"#;
        let _task = TaskParser::parse(Rule::task, buf).unwrap().next().unwrap();
    }

    #[test]
    fn parse_task_fn() {
        let buf = r#"task PrintEnv {
                script {
                    inline = 'env'
                }
            }"#;
        let task = Task::from_str(buf).expect("Failed to parse the task");
        assert_eq!(task.name, "PrintEnv");

        let script = task.script;

        assert_eq!(script.as_bytes(None).unwrap(), "env".as_bytes());
    }

    #[test]
    fn parse_task_fn_with_triple_quotes() {
        let buf = r#"task PrintEnv {
                script {
                    inline = '''env'''
                }
            }"#;
        let task = Task::from_str(buf).expect("Failed to parse the task");
        assert_eq!(task.name, "PrintEnv");

        let script = task.script;
        assert_eq!(script.as_bytes(None).unwrap(), "env".as_bytes());
    }
}
