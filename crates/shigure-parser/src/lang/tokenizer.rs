use crate::lang::ast::*;
use std::{cell::RefCell, iter::Peekable, str::Chars};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum TokenizerErr {
    UnterminatedStringLiteral,
    UnexpectedToken,
    EmptyElementIdentifier,
    InvalidElementIdentifier,
}

pub type TokenResult = Result<Token, TokenizerErr>;
pub type TokenizationResult = Result<(), TokenizerErr>;

pub struct Tokenizer<'a> {
    itr: Peekable<Chars<'a>>,
    pending: RefCell<Option<Token>>,
    current_idx: u32,
    full_idx_count: u32,
}

const MAX_IDX_VALUE: u32 = u32::MAX;

impl<'a> Tokenizer<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            itr: input.chars().peekable(),
            pending: RefCell::new(None),
            current_idx: 0,
            full_idx_count: 0,
        }
    }

    fn lex_number_literal(&mut self) -> TokenResult {
        let mut loc = TokenLoc {
            starts_at: self.current_idx,
            len: 0,
        };

        let mut literal = String::new();
        let mut len = 0;
        while let Some(&c) = self.itr.peek() {
            self.advance();
            if c.is_digit(10) {
                literal.push(c);
                len += 1;
            } else {
                break;
            }
        }

        loc.len = len;
        Ok(Token {
            loc,
            con: TokenContent::Literal(TokenLiteral::NumberLiteral(literal)),
        })
    }

    fn lex_string_literal(&mut self) -> TokenResult {
        let mut literal = String::new();
        let mut loc = TokenLoc {
            starts_at: self.current_idx,
            len: 0,
        };

        let mut quotation_mark_count = 0;
        while let Some(c) = self.next_char() {
            if c == '\"' {
                if quotation_mark_count >= 2 {
                    break;
                }
                quotation_mark_count += 1;
            }
            literal.push(c);
        }

        if quotation_mark_count < 2 {
            Err(TokenizerErr::UnterminatedStringLiteral)
        } else {
            loc.len = self.current_idx - loc.starts_at + 1;
            Ok(Token {
                loc,
                con: TokenContent::Literal(TokenLiteral::StringLiteral(literal)),
            })
        }
    }

    fn lex_reserved(&mut self) -> Option<TokenResult> {
        let mut word = String::new();
        let mut loc = TokenLoc {
            starts_at: self.current_idx,
            len: 0,
        };
        while let Some(c) = self.next_char() {
            if c.is_alphabetic() {
                word.push(c);
                if let Ok(con) = TokenContent::try_from(word.as_str()) {
                    loc.len = self.current_idx - loc.starts_at + 1;
                    return Some(Ok(Token { loc, con }));
                };
            } else {
                self.pending.replace(Some(Token {
                    loc,
                    con: TokenContent::Identifier(word),
                }));

                return None;
            }
        }

        self.pending.replace(Some(Token {
            loc,
            con: TokenContent::Identifier(word),
        }));

        None
    }

    fn lex_identifier(&mut self) -> TokenResult {
        let mut word = String::new();
        let mut loc = TokenLoc {
            starts_at: self.current_idx,
            len: 0,
        };

        if let Some(pending) = self.pending.take() {
            loc = pending.loc;

            match &pending.con {
                TokenContent::Identifier(s) => {
                    word = s.to_string();
                }
                _ => {
                    return Err(TokenizerErr::UnexpectedToken);
                }
            }
        };

        while let Some(&c) = self.itr.peek() {
            if c.is_whitespace() {
                break;
            } else if c == ';' {
                break;
            }

            word.push(c);
            self.advance();
        }

        loc.len = self.current_idx - loc.starts_at + 1;

        Ok(Token {
            loc,
            con: TokenContent::Identifier(word),
        })
    }

    fn lex_alphabetical_chars(&mut self) -> TokenResult {
        if let Some(token) = self.lex_reserved() {
            return token;
        } else {
            return self.lex_identifier();
        }
    }

    fn lex_anchor(&mut self) -> TokenResult {
        if let Some(&c) = self.itr.peek() {
            let mut loc = TokenLoc {
                starts_at: self.current_idx,
                len: 0,
            };
            if c != '#' {
                return Err(TokenizerErr::InvalidElementIdentifier);
            }

            let mut identifier = String::new();

            identifier.push(c);
            self.advance();
            loc.len += 1;

            while let Some(&c) = self.itr.peek() {
                if c.is_whitespace() {
                    break;
                } else if c.is_alphabetic() {
                    identifier.push(c);
                } else {
                    break;
                }

                self.advance();
                loc.len += 1;
            }

            if identifier.is_empty() {
                return Err(TokenizerErr::EmptyElementIdentifier);
            }

            return Ok(Token {
                loc,
                con: TokenContent::Anchor(identifier),
            });
        } else {
            return Err(TokenizerErr::InvalidElementIdentifier);
        }
    }

    fn peek(&mut self) -> Option<&char> {
        self.itr.peek()
    }

    fn advance(&mut self) {
        self.next_char();
    }

    fn next_char(&mut self) -> Option<char> {
        self.current_idx += 1;

        if self.current_idx == MAX_IDX_VALUE {
            self.full_idx_count += 1;
            self.current_idx = 0;
        }

        self.itr.next()
    }

    fn set_pending(&mut self, token: Token) -> TokenizationResult {
        assert!(self.pending.replace(Some(token)).is_none());
        Ok(())
    }

    fn set_pending_or_err(&mut self, res: TokenResult) -> TokenizationResult {
        match res {
            Ok(token) => self.set_pending(token),
            Err(err) => Err(err),
        }
    }

    fn tokenize_char(&mut self, c: char) -> TokenizationResult {
        match c {
            'a'..='z' | 'A'..='Z' => {
                let res = self.lex_alphabetical_chars();
                self.set_pending_or_err(res)
            }
            '0'..='9' => {
                let res = self.lex_number_literal();
                self.set_pending_or_err(res)
            }
            '<' => {
                // ViewElement starting tag
                let loc = TokenLoc {
                    starts_at: self.current_idx,
                    len: 1,
                };
                let con = TokenContent::TagAngleBracketLeft;

                self.set_pending(Token { loc, con })
            }
            '>' => {
                // ViewElement starting tag
                let loc = TokenLoc {
                    starts_at: self.current_idx,
                    len: 1,
                };
                let con = TokenContent::TagAngleBracketRight;

                self.set_pending(Token { loc, con })
            }
            '/' => {
                // Self-closing ViewElement tag
                if let Some('>') = self.next_char() {
                    let loc = TokenLoc {
                        starts_at: self.current_idx,
                        len: 2,
                    };
                    let con = TokenContent::TagAngleSelfClosingRight;
                    self.set_pending(Token { loc, con })
                } else {
                    Err(TokenizerErr::UnexpectedToken)
                }
            }
            '"' => {
                let res = self.lex_string_literal();
                self.set_pending_or_err(res)
            }
            '#' => {
                let res = self.lex_anchor();
                self.set_pending_or_err(res)
            }
            _ => Err(TokenizerErr::UnexpectedToken),
        }
    }

    pub fn next(&mut self) -> Option<TokenResult> {
        while let Some(&c) = self.itr.peek() {
            if c.is_whitespace() {
                self.advance();
                continue;
            }

            match self.tokenize_char(c) {
                Ok(..) => {
                    if let Some(token) = self.pending.take() {
                        return Some(Ok(token));
                    } else {
                        panic!("no pending token")
                    }
                }
                Err(err) => {
                    return Some(Err(err));
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod test {
    use super::*;

    struct Tester<'a> {
        name: &'a str,
        expected: Vec<Token>,
        query: &'a str,
    }

    enum TesterErr {
        TokenizerError,
        UnexpectedToken,
    }

    type TesterResult = Result<(), TesterErr>;

    impl<'a> Tester<'a> {
        pub fn new(name: &'a str, expected: Vec<Token>, query: &'a str) -> Self {
            Self {
                name,
                expected,
                query,
            }
        }

        pub fn run(&self) -> TesterResult {
            let mut tokenizer = Tokenizer::new(self.query);
            let mut i: usize = 0;
            while let Some(token) = tokenizer.next() {
                match token {
                    Ok(token) => {
                        if token != self.expected[i] {
                            println!("{}: Failed with unexpected token\n- Expected:\n{:?}\n- Result:\n{:?}", self.name, self.expected[i], token);
                            return Err(TesterErr::UnexpectedToken);
                        }
                    }
                    Err(err) => {
                        println!(
                            "{}: Failed with tokenizer error\n- Error:\n{:?}",
                            self.name, err
                        );
                        return Err(TesterErr::TokenizerError);
                    }
                }
                i += 1;
            }
            println!("{}: Passed", self.name);
            Ok(())
        }
    }

    struct MultiTester<'a> {
        tests: Vec<Tester<'a>>,
    }

    impl<'a> MultiTester<'a> {
        pub fn new() -> Self {
            Self { tests: Vec::new() }
        }

        pub fn add_test(&mut self, test: Tester<'a>) {
            self.tests.push(test);
        }

        pub fn run_all(&mut self) {
            for test in &self.tests {
                assert!(test.run().is_ok());
            }
        }
    }

    #[test]
    fn decimal_digits() {
        assert!(Tester::new(
            "numeric literals",
            vec![Token {
                loc: TokenLoc {
                    starts_at: 0,
                    len: 2,
                },
                con: TokenContent::Literal(TokenLiteral::NumberLiteral("91".to_string())),
            }],
            "91",
        )
        .run()
        .is_ok());
    }

    #[test]
    fn multiple_tokens() {
        assert!(Tester::new(
            "multiple tokens",
            vec![
                Token {
                    loc: TokenLoc {
                        starts_at: 0,
                        len: 1,
                    },
                    con: TokenContent::Identifier("x".to_string()),
                },
                Token {
                    loc: TokenLoc {
                        starts_at: 2,
                        len: 2,
                    },
                    con: TokenContent::Literal(TokenLiteral::NumberLiteral("91".to_string())),
                }
            ],
            "x 91",
        )
        .run()
        .is_ok());
    }

    #[test]
    fn string_literal() {
        assert!(Tester::new(
            "string literal",
            vec![Token {
                loc: TokenLoc {
                    starts_at: 0,
                    len: 14,
                },
                con: TokenContent::Literal(TokenLiteral::StringLiteral(
                    "\"hello, world\"".to_string()
                )),
            }],
            "\"hello, world\"",
        )
        .run()
        .is_ok());
    }

    #[test]
    fn lex_queries() {
        let mut tester = MultiTester::new();
        tester.add_test(Tester::new(
            "view",
            vec![
                Token {
                    loc: TokenLoc {
                        starts_at: 0,
                        len: 1,
                    },
                    con: TokenContent::TagAngleBracketLeft,
                },
                Token {
                    loc: TokenLoc {
                        starts_at: 1,
                        len: 7,
                    },
                    con: TokenContent::Identifier("Element".into()),
                },
                Token {
                    loc: TokenLoc {
                        starts_at: 8,
                        len: 7,
                    },
                    con: TokenContent::Anchor("Anchor".into()),
                },
                Token {
                    loc: TokenLoc {
                        starts_at: 16,
                        len: 2,
                    },
                    con: TokenContent::TagAngleSelfClosingRight,
                },
            ],
            "<Element#anchor />",
        ));

        tester.run_all();
    }
}