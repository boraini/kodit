use super::line::Line;
use super::lexing_specification::LexingSpecification;

pub fn lex_line(line : &Line, specs : &[Box<dyn LexingSpecification>]) -> Result<Line, String> {
    for spec in specs {
        match spec.lex(line) {
            Some(line) => return Ok(line),
            None => (),
        }
    }

    return Err(format!("Couldn't lex line {:?}", line));
}

pub fn lex(lines : &[Line], specs : &[Box<dyn LexingSpecification>]) -> Result<Vec<Line>, String> {
    lines.iter().map(|line| lex_line(line, specs)).collect()
}