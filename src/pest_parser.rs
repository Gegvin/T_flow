use pest::Parser as PestParserTrait;
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "tflow_pest.pest"]
struct TFlowPestParser;

pub fn parse_pest(source: &str) -> Result<(), String> {
    TFlowPestParser::parse(Rule::program, source)
        .map(|_| ())
        .map_err(|error| error.to_string())
}
