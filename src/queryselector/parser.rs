#[cfg(not(feature = "std"))]
use crate::ParseError;
use crate::{stream::Stream, util};

use super::Selector;

/// A query selector parser
pub struct Parser<'a> {
    stream: Stream<'a, u8>,
}

impl<'a> Parser<'a> {
    /// Creates a new query selector parser
    pub fn new(input: &'a [u8]) -> Self {
        Self {
            stream: Stream::new(input),
        }
    }

    #[cfg(not(feature = "std"))]
    fn skip_whitespaces(&mut self) -> bool {
        let has_whitespace = self.stream.expect_and_skip_cond(b' ');
        while !self.stream.is_eof() {
            if self.stream.expect_and_skip(b' ').is_none() {
                break;
            }
        }
        has_whitespace
    }

    fn read_identifier(&mut self) -> &'a [u8] {
        let start = self.stream.idx;

        while !self.stream.is_eof() {
            let is_ident = self.stream.current().copied().is_some_and(util::is_ident);
            if !is_ident {
                break;
            } else {
                self.stream.advance();
            }
        }

        self.stream.slice(start, self.stream.idx)
    }

    #[cfg(feature = "std")]
    fn read_css_identifier(&mut self) -> &'a [u8] {
        let start = self.stream.idx;
        while self
            .stream
            .current_cpy()
            .is_some_and(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_' || c >= 128)
        {
            self.stream.advance();
        }
        self.stream.slice(start, self.stream.idx)
    }

    #[cfg(not(feature = "std"))]
    fn parse_combinator<const MAX_SELECTOR_NODES: usize>(
        &mut self,
        left: Selector<'a, MAX_SELECTOR_NODES>,
    ) -> Result<Selector<'a, MAX_SELECTOR_NODES>, ParseError> {
        let has_whitespaces = self.skip_whitespaces();
        if self.stream.current_cpy().is_none() {
            return Ok(left);
        }
        if has_whitespaces || matches!(self.stream.current_cpy(), Some(b',' | b'>')) {
            return Err(ParseError::SelectorCapacityExceeded);
        }
        Err(ParseError::SelectorCapacityExceeded)
    }

    fn parse_attribute<const MAX_SELECTOR_NODES: usize>(
        &mut self,
    ) -> Option<Selector<'a, MAX_SELECTOR_NODES>> {
        let attribute = self.read_identifier();
        if attribute.is_empty() {
            return None;
        }
        let ty = match self.stream.current_cpy() {
            Some(b']') => {
                self.stream.advance();
                Selector::Attribute(attribute)
            }
            Some(b'=') => {
                self.stream.advance();
                let quote = self.stream.expect_oneof_and_skip(b"\"'");
                let value = if let Some(q) = quote {
                    let start = self.stream.idx;
                    while self
                        .stream
                        .current_cpy()
                        .is_some_and(|c| c != q && c != b'\\')
                    {
                        self.stream.advance();
                    }
                    self.stream.slice(start, self.stream.idx)
                } else {
                    self.read_identifier()
                };
                if let Some(quote) = quote {
                    // Only require the given quote if the value starts with a quote
                    self.stream.expect_and_skip(quote)?;
                }
                self.stream.expect_and_skip(b']')?;
                Selector::AttributeValue(attribute, value)
            }
            Some(c @ b'~' | c @ b'^' | c @ b'$' | c @ b'*') => {
                self.stream.advance();
                self.stream.expect_and_skip(b'=')?;
                let quote = self.stream.expect_oneof_and_skip(b"\"'");
                let value = if let Some(q) = quote {
                    let start = self.stream.idx;
                    while self
                        .stream
                        .current_cpy()
                        .is_some_and(|c| c != q && c != b'\\')
                    {
                        self.stream.advance();
                    }
                    self.stream.slice(start, self.stream.idx)
                } else {
                    self.read_identifier()
                };
                if let Some(quote) = quote {
                    // Only require the given quote if the value starts with a quote
                    self.stream.expect_and_skip(quote)?;
                }
                self.stream.expect_and_skip(b']')?;
                match c {
                    b'~' => Selector::AttributeValueWhitespacedContains(attribute, value),
                    b'^' => Selector::AttributeValueStartsWith(attribute, value),
                    b'$' => Selector::AttributeValueEndsWith(attribute, value),
                    b'*' => Selector::AttributeValueSubstring(attribute, value),
                    _ => unreachable!(),
                }
            }
            _ => return None,
        };
        Some(ty)
    }

    /// Parses a full selector, respecting compound, combinator and list precedence.
    #[cfg(feature = "std")]
    pub fn selector(&mut self) -> Option<Selector<'a>> {
        css(self.stream.data())
    }

    /// Parses a full selector without allocation.
    #[cfg(not(feature = "std"))]
    pub fn selector<const MAX_SELECTOR_NODES: usize>(
        &mut self,
    ) -> Result<Selector<'a, MAX_SELECTOR_NODES>, ParseError> {
        self.skip_whitespaces();
        let tok = self
            .stream
            .current_cpy()
            .ok_or(ParseError::SelectorCapacityExceeded)?;

        let left = match tok {
            b'#' => {
                self.stream.advance();
                let id = self.read_identifier();
                Selector::Id(id)
            }
            b'.' => {
                self.stream.advance();
                let class = self.read_identifier();
                Selector::Class(class)
            }
            b'*' => {
                self.stream.advance();
                Selector::All
            }
            b'[' => {
                self.stream.advance();
                self.parse_attribute::<MAX_SELECTOR_NODES>()
                    .ok_or(ParseError::SelectorCapacityExceeded)?
            }
            _ if util::is_ident(tok) => {
                let tag = self.read_identifier();
                Selector::Tag(tag)
            }
            _ => return Err(ParseError::SelectorCapacityExceeded),
        };

        self.parse_combinator(left)
    }
}

#[cfg(feature = "std")]
fn css(input: &[u8]) -> Option<Selector<'_>> {
    css_at(input, 0)
}

#[cfg(feature = "std")]
fn css_at(input: &[u8], depth_limit: usize) -> Option<Selector<'_>> {
    if depth_limit >= 64 {
        return None;
    }
    let input = trim(input);
    if input.is_empty() {
        return None;
    }
    // Split only outside attributes, functional pseudo-classes and quoted strings.
    let mut depth = 0usize;
    let mut quote = None;
    let mut relations = Vec::new();
    let mut commas = Vec::new();
    for (i, &c) in input.iter().enumerate() {
        if let Some(q) = quote {
            if c == q {
                quote = None;
            }
            continue;
        }
        match c {
            b'\'' | b'"' => quote = Some(c),
            b'[' | b'(' => depth += 1,
            b']' | b')' => depth = depth.checked_sub(1)?,
            b',' if depth == 0 => commas.push(i),
            b'>' | b'+' | b'~' if depth == 0 => relations.push((i, c)),
            c if c.is_ascii_whitespace() && depth == 0 => relations.push((i, b' ')),
            _ => {}
        }
    }
    if depth != 0 || quote.is_some() {
        return None;
    }
    if let Some(&i) = commas.first() {
        return Some(Selector::Or(
            Box::new(css_at(&input[..i], depth_limit + 1)?),
            Box::new(css_at(&input[i + 1..], depth_limit + 1)?),
        ));
    }
    // Rightmost relation gives left-associative chains; spaces surrounding an
    // explicit combinator are not descendant combinators.
    for &(i, c) in relations.iter().rev() {
        let left = trim(&input[..i]);
        let right = trim(&input[i + 1..]);
        if c == b' '
            && (left.is_empty()
                || right.is_empty()
                || left.last().is_some_and(|c| b">+~".contains(c))
                || right.first().is_some_and(|c| b">+~".contains(c)))
        {
            continue;
        }
        let left = Box::new(css_at(left, depth_limit + 1)?);
        let right = Box::new(css_at(right, depth_limit + 1)?);
        return Some(match c {
            b'>' => Selector::Parent(left, right),
            b'+' => Selector::Adjacent(left, right),
            b'~' => Selector::Sibling(left, right),
            _ => Selector::Descendant(left, right),
        });
    }
    let mut parser = Parser::new(input);
    let mut result = None;
    let mut compounds = 0;
    while let Some(tok) = parser.stream.current_cpy() {
        compounds += 1;
        if compounds >= 64 {
            return None;
        }
        let simple = match tok {
            b'#' | b'.' => {
                parser.stream.advance();
                let ident = parser.read_css_identifier();
                if ident.is_empty() {
                    return None;
                }
                if tok == b'#' {
                    Selector::Id(ident)
                } else {
                    Selector::Class(ident)
                }
            }
            b'*' => {
                parser.stream.advance();
                Selector::All
            }
            b'[' => {
                parser.stream.advance();
                parser.parse_attribute::<0>()?
            }
            b':' => {
                parser.stream.advance();
                let name = parser.read_css_identifier();
                match name {
                    b"first-child" => Selector::NthChild(0, 1),
                    b"nth-child" | b"not" | b"has" => {
                        parser.stream.expect_and_skip(b'(')?;
                        let start = parser.stream.idx;
                        let mut depth = 1;
                        let mut quote = None;
                        while let Some(c) = parser.stream.current_cpy() {
                            if let Some(q) = quote {
                                if c == q {
                                    quote = None;
                                }
                            } else {
                                match c {
                                    b'\'' | b'"' => quote = Some(c),
                                    b'(' => depth += 1,
                                    b')' => {
                                        depth -= 1;
                                        if depth == 0 {
                                            break;
                                        }
                                    }
                                    _ => {}
                                }
                            }
                            parser.stream.advance();
                        }
                        let arg = trim(parser.stream.slice(start, parser.stream.idx));
                        parser.stream.expect_and_skip(b')')?;
                        match name {
                            b"not" => Selector::Not(Box::new(css_at(arg, depth_limit + 1)?)),
                            b"has" => Selector::Has(Box::new(css_at(arg, depth_limit + 1)?)),
                            _ => {
                                let text = core::str::from_utf8(arg).ok()?;
                                let (a, b) = match text {
                                    "odd" => (2, 1),
                                    "even" => (2, 0),
                                    _ if text.contains('n') => {
                                        let (a, b) = text.split_once('n')?;
                                        let a = match a.trim() {
                                            "" | "+" => 1,
                                            "-" => -1,
                                            a => a.parse::<i32>().ok()?,
                                        };
                                        let b = b.trim();
                                        let b = if b.is_empty() {
                                            0
                                        } else {
                                            if !b.starts_with('+') && !b.starts_with('-') {
                                                return None;
                                            }
                                            let compact: String = b
                                                .chars()
                                                .filter(|c| !c.is_ascii_whitespace())
                                                .collect();
                                            compact.parse::<i32>().ok()?
                                        };
                                        (a, b)
                                    }
                                    _ => (0, text.parse::<i32>().ok()?),
                                };
                                Selector::NthChild(a, b)
                            }
                        }
                    }
                    _ => return None,
                }
            }
            _ if tok.is_ascii_alphanumeric() || tok == b'_' || tok == b'-' || tok >= 128 => {
                // A type selector must start a compound selector.
                if result.is_some() {
                    return None;
                }
                Selector::Tag(parser.read_css_identifier())
            }
            _ => return None,
        };
        result = Some(match result {
            None => simple,
            Some(left) => Selector::And(Box::new(left), Box::new(simple)),
        });
    }
    result
}

#[cfg(feature = "std")]
fn trim(mut input: &[u8]) -> &[u8] {
    while input.first().is_some_and(u8::is_ascii_whitespace) {
        input = &input[1..];
    }
    while input.last().is_some_and(u8::is_ascii_whitespace) {
        input = &input[..input.len() - 1];
    }
    input
}
