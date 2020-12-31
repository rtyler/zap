use log::*;
use pest::error::Error as PestError;
use pest::error::ErrorVariant;
use pest::iterators::Pairs;
use pest::Parser;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Parser)]
#[grammar = "plan.pest"]
struct PlanParser;

#[derive(Clone, Debug)]
pub struct ExecutableTask {
    pub task: crate::task::Task,
    pub parameters: HashMap<String, String>,
}

impl ExecutableTask {
    pub fn new(task: crate::task::Task, parameters: HashMap<String, String>) -> Self {
        Self { task, parameters }
    }
}

#[derive(Clone, Debug)]
pub struct Plan {
    pub tasks: Vec<ExecutableTask>,
}

impl Plan {
    pub fn new() -> Self {
        Self { tasks: vec![] }
    }

    pub fn from_str(buf: &str) -> Result<Self, PestError<Rule>> {
        let mut parser = PlanParser::parse(Rule::planfile, buf)?;
        let mut plan = Plan::new();

        while let Some(parsed) = parser.next() {
            match parsed.as_rule() {
                Rule::task => {
                    let mut raw_task = None;
                    let mut parameters: HashMap<String, String> = HashMap::new();

                    for pair in parsed.into_inner() {
                        match pair.as_rule() {
                            Rule::string => {
                                let path = PathBuf::from(parse_str(&mut pair.into_inner())?);

                                match crate::task::Task::from_path(&path) {
                                    Ok(task) => raw_task = Some(task),
                                    Err(err) => {
                                        error!("Failed to parse task: {:?}", err);
                                    }
                                }
                            }
                            Rule::kwarg => {
                                let (key, val) = parse_kwarg(&mut pair.into_inner())?;
                                parameters.insert(key, val);
                            }
                            _ => {}
                        }
                    }

                    if let Some(task) = raw_task {
                        plan.tasks.push(ExecutableTask::new(task, parameters));
                    }
                }
                _ => {}
            }
        }

        Ok(plan)
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

fn parse_kwarg(parser: &mut Pairs<Rule>) -> Result<(String, String), PestError<Rule>> {
    let mut identifier = None;
    let mut arg = None;

    while let Some(parsed) = parser.next() {
        match parsed.as_rule() {
            Rule::identifier => identifier = Some(parsed.as_str().to_string()),
            Rule::arg => arg = Some(parse_str(&mut parsed.into_inner())?),
            _ => {}
        }
    }

    if identifier.is_some() && arg.is_some() {
        return Ok((identifier.unwrap(), arg.unwrap()));
    }
    Err(PestError::new_from_pos(
        ErrorVariant::CustomError {
            message: "Could not parse keyword arguments for parameters".to_string(),
        },
        /* TODO: Find a better thing to report */
        pest::Position::from_start(""),
    ))
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
            Rule::single_quoted => {
                return parse_str(&mut parsed.into_inner());
            }
            Rule::inner_single_str => {
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
    fn parse_simple_plan() {
        let buf = r#"/*
 * This zplan just loads a couple tasks and then executes them
 *
 * It is expected to be run from the root of the project tree.
 */

task '../tasks/echo.ztask' {
    msg = 'Hello from the wonderful world of zplans!'
}

task '../tasks/echo.ztask' {
    msg = 'This can actually take inline shells too: $(date)'
}"#;
        let _plan = PlanParser::parse(Rule::planfile, buf)
            .unwrap()
            .next()
            .unwrap();
    }

    #[test]
    fn parse_plan_fn() {
        let buf = r#"task '../tasks/echo.ztask' {
                        msg = 'Hello from the wonderful world of zplans!'
                    }

                    task '../tasks/echo.ztask' {
                        msg = 'This can actually take inline shells too: $(date)'
                    }"#;
        let plan = Plan::from_str(buf).expect("Failed to parse the plan");
        assert_eq!(plan.tasks.len(), 2);
    }
}
