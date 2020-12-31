use pest::Parser;

#[derive(Parser)]
#[grammar = "plan.pest"]
struct PlanParser;

pub struct Plan {
    pub tasks: Vec<crate::task::Task>,
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

task "../tasks/echo.ztask" {
    msg = "Hello from the wonderful world of zplans!"
}

task "../tasks/echo.ztask" {
    msg = "This can actually take inline shells too: $(date)"
}"#;
        let _plan = PlanParser::parse(Rule::planfile, buf).unwrap().next().unwrap();
    }
}
