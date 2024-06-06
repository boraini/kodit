use super::line::Line;

pub mod v0;

pub enum Matcher {
    Symbol(String),
    Argument(i32),
    Rest,
}

pub trait LexingSpecification {
    fn lex(&self, line : &Line) -> Option<Line>;
}