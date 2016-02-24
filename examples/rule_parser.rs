//! http parser comparable to the http-parser found in attoparsec's examples.
//!
//! Reads data in the following format:
//!
//! ```text
//! GET /robot.txt HTTP/1.1
//! Host: localhost
//! Accept: text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8
//!
//! ```

#[macro_use]
extern crate chomp;

use std::fs::File;
use std::env;

use chomp::*;

use chomp::buffer::{Source, Stream, StreamError};


pub struct Rule {
    pub src: String,
    pub dest: String,
    pub constraint: Constraint
}

#[derive(Debug)]
pub enum Constraint {
    Id(String),
    And(Box<Constraint>, Box<Constraint>),
    Or(Box<Constraint>, Box<Constraint>),
    Not(Box<Constraint>)
}

fn is_horizontal_space(c: u8) -> bool { c == b' ' || c == b'\t' }
fn is_space(c: u8)            -> bool { c == b' ' }
fn is_not_space(c: u8)        -> bool { c != b' ' }
fn is_end_of_line(c: u8)      -> bool { c == b'\r' || c == b'\n' }

fn is_identifier_char(c: u8)  -> bool { match c { b'A'...b'z' => true, _ => false } }

fn end_of_line(i: Input<u8>) -> U8Result<u8> {
    or(i, |i| parse!{i;
               token(b'\r');
               token(b'\n');
               ret b'\r'},
          |i| token(i, b'\n'))
}

fn identifier(i: Input<u8>) -> U8Result<Constraint> {
    parse!{i;
                take_while(is_space);
        let n = take_while1(is_identifier_char);

        ret Constraint::Id("identifier".to_string())
    }
}

fn parentheses(i: Input<u8>) -> U8Result<Constraint> {
    parse!{i;
                take_while(is_space);
                token(b'(');
        let c = constraint();
                token(b')');

        ret c
    }
}

fn not(i: Input<u8>) -> U8Result<Constraint> {
    parse!{i;
                take_while(is_space);
                token(b'!');
        let c = constraint();

        ret Constraint::Not(Box::new(c))
    }
}

fn unary(i: Input<u8>) -> U8Result<Constraint> {
    parse!{i;
                not()
                <|> parentheses()
                <|> identifier()
    }
}

fn conjunction(i: Input<u8>) -> U8Result<Constraint> {
    parse!{i;
        let first = unary();
                    take_while(is_space);
                    token(b'.');
        let other = conjunction();

        ret Constraint::Or(
            Box::new(first),
            Box::new(other)
        )
    }
}

fn conjunction_or_unary(i: Input<u8>) -> U8Result<Constraint> {
    parse!{i;
        conjunction()
        <|> unary()
    }
}

fn disjunction(i: Input<u8>) -> U8Result<Constraint> {
    parse!{i;
        let first = conjunction_or_unary();
                    take_while(is_space);
                    token(b'|');
        let other = disjunction();

        ret Constraint::Or(
            Box::new(first),
            Box::new(other)
        )
    }
}

fn binary(i: Input<u8>) -> U8Result<Constraint> {
    parse!{i;
        disjunction()
        <|> conjunction()
    }
}

fn constraint(i: Input<u8>) -> U8Result<Constraint> {
    parse!{i;
        binary()
        <|> unary()
    }
}

fn rule(i: Input<u8>) -> U8Result<Rule> {
    parse!{i;
                  take_while(is_space);
        let src = take_while1(is_identifier_char);
                  take_while(is_space);
                  token(b':');
        let c   = constraint();
                  token(b':');
                  take_while(is_space);
        let des = take_while1(is_identifier_char);
                  take_while(is_space);
                  take_while(is_end_of_line);

        ret Rule { src: "hithere".to_string(), dest: "hithere".to_string(), constraint: Constraint::Id("hithere".to_string()) }
    }
}

#[allow(dead_code)]
fn print_all_rules(rules: Vec<Rule>) {
    println!("\n");
    for rule in rules {
        println!("{} == [constraint] ==> {}", rule.src, rule.dest);
        println!("{:?}\n", rule.constraint);
    }
}

fn main() {
    let file  = File::open(env::args().nth(1).expect("File to read")).ok().expect("Failed to open file");
    // Use default buffer settings for a Read source
    let mut i = Source::new(file);

    let mut rules = Vec::<Rule>::new();

    loop {
        match i.parse(rule) {
            Ok(rule)                        => rules.push(rule),
            Err(StreamError::Retry)      => {}, // Needed to refill buffer when necessary
            Err(StreamError::EndOfInput) => break,
            Err(e)                       => { panic!("{:?}", e); }
        };
    }

    print_all_rules(rules);
}