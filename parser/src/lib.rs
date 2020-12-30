#[macro_use]
extern crate pest;
#[macro_use]
extern crate pest_derive;

use pest::Parser;

#[derive(Parser)]
#[grammar="task.pest"]
struct TaskParser;

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
        let _task = TaskParser::parse(Rule::task, buf)
            .unwrap().next().unwrap();
    }

    #[test]
    fn parse_no_parameters() {
        let buf = r#"task PrintEnv {
                script {
                    inline = "env"
                }
            }"#;
        let _task = TaskParser::parse(Rule::task, buf)
            .unwrap().next().unwrap();
    }
}
