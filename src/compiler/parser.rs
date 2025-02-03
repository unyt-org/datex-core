use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "datex.pest"]
pub struct DatexParser;
