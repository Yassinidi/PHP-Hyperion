//! PHP Lexer implementation

#[derive(Debug, PartialEq, Clone)]
pub enum Token {
    // Keywords
    If,
    Elseif,
    Else,
    Echo,
    Function,
    Return,
    Class,
    New,
    While,
    For,
    Fn,
    Foreach,
    As,
    Yield,
    NullCoalesce,
    NullsafeObjectOperator,
    Ellipsis,
    Switch,
    Match,
    Case,
    Default,
    Break,
    Continue,
    Use,
    Require,
    Include,
    RequireOnce,
    IncludeOnce,
    Try,
    Catch,
    Finally,
    Throw,
    Namespace,
    Trait,
    Interface,
    Implements,
    Public,
    Protected,
    Private,
    Readonly,
    Static,
    Self_,
    Extends,
    Enum,
    Clone,
    Declare,
    Final,
    Abstract,
    Const,
    InstanceOf,
    Isset,
    Empty,
    Unset,
    List,
    Print,
    Eval,
    Do,
    Global,
    Endif,
    Endwhile,
    Endfor,
    Endforeach,
    Endswitch,
    // Identifiers and Literals
    Variable(String),
    Identifier(String),
    Integer(i64),
    Float(f64),
    StringLiteral(String),
    
    // Operators
    Plus,
    Minus,
    Multiply,
    Divide,
    Modulo,         // %
    Dot,
    Assign,        // =
    Equals,        // ==
    StrictEquals,  // ===
    NotEquals,     // !=
    StrictNotEquals, // !==
    LessThan,      // <
    GreaterThan,   // >
    LessThanOrEqual, // <=
    GreaterThanOrEqual, // >=
    ObjectOperator, // ->
    FatArrow,      // =>
    Spaceship,     // <=>
    LogicalAnd,    // &&
    LogicalOr,     // ||
    NullCoalesceAssign, // ??=
    DotAssign,     // .=
    PlusAssign,    // +=
    MinusAssign,   // -=
    MultiplyAssign,// *=
    DivideAssign,  // /=
    ModuloAssign,  // %=
    PlusPlus,      // ++
    MinusMinus,    // --
    Not,           // !
    
    // Symbols
    OpenParen,
    CloseParen,
    OpenBrace,
    CloseBrace,
    OpenBracket,
    CloseBracket,
    AttributeOpen, // #[
    Semicolon,
    Comma,
    Colon,
    DoubleColon, // ::
    Pipe,        // |
    Ampersand,   // &
    QuestionMark, // ?
    At,          // @
    Power,       // **
    BitwiseAndAssign, // &=
    BitwiseOrAssign,  // |=
    BitwiseXorAssign, // ^=
    ShiftLeftAssign,  // <<=
    ShiftRightAssign, // >>=
    PowerAssign,      // **=
    ShiftLeft,        // <<
    ShiftRight,       // >>
    BitwiseXor,       // ^
    
    // Comments and whitespace
    Whitespace(String),
    Comment(String),
    DocComment(String),

    // Special
    OpenTag, // <?php
    CloseTag, // ?>
    InlineHtml(String),
    Eof,
    Error(String),
    InterpolatedString(Vec<InterpolatedPart>),
}

#[derive(Debug, PartialEq, Clone)]
pub enum InterpolatedPart {
    Literal(String),
    Variable(String),
    Expression(String),
}

#[derive(Debug, PartialEq, Clone)]
pub struct TokenRecord {
    pub token: Token,
    pub line: usize,
    pub column: usize,
    pub raw: Option<String>,
}

#[derive(Clone)]
pub struct Lexer<'a> {
    source: &'a str,
    cursor: usize,
    chars: std::iter::Peekable<std::str::Chars<'a>>,
    pub line: usize,
    pub column: usize,
    pending_token: Option<TokenRecord>,
    pub in_php_mode: bool,
    pub preserve_whitespace: bool,
    pub preserve_comments: bool,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Self {
        let trimmed_source = if source.starts_with("#!") {
            if let Some(pos) = source.find('\n') {
                &source[pos + 1..]
            } else {
                ""
            }
        } else {
            source
        };
        Self {
            source: trimmed_source,
            cursor: 0,
            chars: trimmed_source.chars().peekable(),
            line: 1,
            column: 1,
            pending_token: None,
            in_php_mode: false,
            preserve_whitespace: false,
            preserve_comments: false,
        }
    }

    fn advance(&mut self) -> Option<char> {
        let c = self.chars.next();
        if let Some(ch) = c {
            self.cursor += ch.len_utf8();
            if ch == '\n' {
                self.line += 1;
                self.column = 1;
            } else {
                self.column += 1;
            }
        }
        c
    }

    fn peek(&mut self) -> Option<&char> {
        self.chars.peek()
    }

    /// Look ahead `n` characters without consuming (0 == same as `peek`).
    fn peek_at(&self, n: usize) -> Option<char> {
        self.chars.clone().nth(n)
    }

    fn skip_whitespace(&mut self) {
        loop {
            let mut cloned_chars = self.chars.clone();
            let c1 = cloned_chars.next();
            let c2 = cloned_chars.next();

            if let Some(&c) = self.peek() {
                if c.is_whitespace() {
                    self.advance();
                    continue;
                }
            }

            if c1 == Some('/') && c2 == Some('/') {
                // skip // comments
                self.advance(); // consume /
                self.advance(); // consume /
                while let Some(c) = self.advance() {
                    if c == '\n' {
                        break;
                    }
                }
                continue;
            }

            if c1 == Some('/') && c2 == Some('*') {
                // skip /* comments
                self.advance(); // consume /
                self.advance(); // consume *
                let mut prev = '\0';
                while let Some(c) = self.advance() {
                    if prev == '*' && c == '/' {
                        break;
                    }
                    prev = c;
                }
                continue;
            }

            if c1 == Some('#') && c2 != Some('[') { // #[ is an attribute, not a comment
                // skip # comments
                self.advance(); // consume #
                while let Some(c) = self.advance() {
                    if c == '\n' {
                        break;
                    }
                }
                continue;
            }
            
            break;
        }
    }

    pub fn next_token(&mut self) -> TokenRecord {
        // Return any pending token first (e.g. from elseif split)
        if let Some(pending) = self.pending_token.take() {
            return pending;
        }

        if !self.in_php_mode {
            let start_line = self.line;
            let start_column = self.column;
            let start_offset = self.cursor;
            let mut html = String::new();
            
            while let Some(&c) = self.peek() {
                if c == '<' {
                    if self.peek_at(1) == Some('?') {
                        // We found an open tag.
                        if html.is_empty() {
                            // If html is empty, we just parse the OpenTag directly, and switch mode.
                            self.in_php_mode = true;
                            // Let the normal tokenizer handle the open tag.
                            break;
                        } else {
                            // Return the html we collected.
                            return TokenRecord {
                                token: Token::InlineHtml(html),
                                line: start_line,
                                column: start_column,
                                raw: Some(self.source[start_offset..self.cursor].to_string()),
                            };
                        }
                    }
                }
                html.push(self.advance().unwrap());
            }

            if !html.is_empty() {
                return TokenRecord {
                    token: Token::InlineHtml(html),
                    line: start_line,
                    column: start_column,
                    raw: Some(self.source[start_offset..self.cursor].to_string()),
                };
            }
            
            // If we get here and html is empty, and we peeked '<', it broke out to let PHP mode handle it.
            // OR we reached EOF.
            if self.peek().is_none() {
                return TokenRecord {
                    token: Token::Eof,
                    line: start_line,
                    column: start_column,
                    raw: None,
                };
            }
        }

        if self.preserve_whitespace || self.preserve_comments {
            let start_line = self.line;
            let start_column = self.column;
            let start_offset = self.cursor;

            if let Some(&c) = self.peek() {
                if c.is_whitespace() && self.preserve_whitespace {
                    let mut ws = String::new();
                    while let Some(&wc) = self.peek() {
                        if wc.is_whitespace() {
                            ws.push(wc);
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    return TokenRecord {
                        token: Token::Whitespace(ws),
                        line: start_line,
                        column: start_column,
                        raw: Some(self.source[start_offset..self.cursor].to_string()),
                    };
                }
            }

            let mut cloned = self.chars.clone();
            let c1 = cloned.next();
            let c2 = cloned.next();
            let c3 = cloned.next();

            if c1 == Some('/') && c2 == Some('/') && self.preserve_comments {
                let mut comm = String::new();
                comm.push(self.advance().unwrap()); // /
                comm.push(self.advance().unwrap()); // /
                while let Some(ch) = self.advance() {
                    comm.push(ch);
                    if ch == '\n' {
                        break;
                    }
                }
                return TokenRecord {
                    token: Token::Comment(comm),
                    line: start_line,
                    column: start_column,
                    raw: Some(self.source[start_offset..self.cursor].to_string()),
                };
            }

            if c1 == Some('#') && c2 != Some('[') && self.preserve_comments {
                let mut comm = String::new();
                comm.push(self.advance().unwrap()); // #
                while let Some(ch) = self.advance() {
                    comm.push(ch);
                    if ch == '\n' {
                        break;
                    }
                }
                return TokenRecord {
                    token: Token::Comment(comm),
                    line: start_line,
                    column: start_column,
                    raw: Some(self.source[start_offset..self.cursor].to_string()),
                };
            }

            if c1 == Some('/') && c2 == Some('*') && self.preserve_comments {
                let is_doc = c3 == Some('*');
                let mut comm = String::new();
                comm.push(self.advance().unwrap()); // /
                comm.push(self.advance().unwrap()); // *
                let mut prev = '\0';
                while let Some(ch) = self.advance() {
                    comm.push(ch);
                    if prev == '*' && ch == '/' {
                        break;
                    }
                    prev = ch;
                }
                return TokenRecord {
                    token: if is_doc { Token::DocComment(comm) } else { Token::Comment(comm) },
                    line: start_line,
                    column: start_column,
                    raw: Some(self.source[start_offset..self.cursor].to_string()),
                };
            }
        } else {
            self.skip_whitespace();
        }

        // Capture starting position of the token
        let start_line = self.line;
        let start_column = self.column;
        let start_offset = self.cursor;

        let token = match self.advance() {
            Some(c) => self.tokenize_char(c),
            None => Token::Eof,
        };

        let raw = if token == Token::Eof {
            None
        } else {
            Some(self.source[start_offset..self.cursor].to_string())
        };

        TokenRecord {
            token,
            line: start_line,
            column: start_column,
            raw,
        }
    }

    fn tokenize_char(&mut self, c: char) -> Token {
        match c {
            '+' => {
                if self.peek() == Some(&'=') {
                    self.advance();
                    Token::PlusAssign
                } else if self.peek() == Some(&'+') {
                    self.advance();
                    Token::PlusPlus
                } else {
                    Token::Plus
                }
            }
            '-' => {
                if self.peek() == Some(&'>') {
                    self.advance();
                    Token::ObjectOperator
                } else if self.peek() == Some(&'=') {
                    self.advance();
                    Token::MinusAssign
                } else if self.peek() == Some(&'-') {
                    self.advance();
                    Token::MinusMinus
                } else {
                    Token::Minus
                }
            }
            '*' => {
                if self.peek() == Some(&'*') {
                    self.advance();
                    if self.peek() == Some(&'=') {
                        self.advance();
                        Token::PowerAssign
                    } else {
                        Token::Power
                    }
                } else if self.peek() == Some(&'=') {
                    self.advance();
                    Token::MultiplyAssign
                } else {
                    Token::Multiply
                }
            }
            '/' => {
                if self.peek() == Some(&'=') {
                    self.advance();
                    Token::DivideAssign
                } else {
                    Token::Divide
                }
            }
            '.' => {
                if self.peek() == Some(&'=') {
                    self.advance();
                    Token::DotAssign
                } else if self.peek() == Some(&'.') {
                    self.advance();
                    if self.peek() == Some(&'.') {
                        self.advance();
                        Token::Ellipsis
                    } else {
                        Token::Dot // just two dots, treat as dot + dot
                    }
                } else if self.peek().is_some_and(|c| c.is_ascii_digit()) {
                    // Leading-dot float: `.5` is 0.5, not concat-then-5.
                    let mut num_str = String::from("0.");
                    let mut has_exponent = false;
                    while let Some(&ch) = self.peek() {
                        if ch.is_ascii_digit() {
                            num_str.push(ch);
                            self.advance();
                        } else if ch == '_' {
                            self.advance();
                        } else if (ch == 'e' || ch == 'E') && !has_exponent {
                            let valid = match self.peek_at(1) {
                                Some(n) if n.is_ascii_digit() => true,
                                Some('+') | Some('-') => {
                                    self.peek_at(2).is_some_and(|c| c.is_ascii_digit())
                                }
                                _ => false,
                            };
                            if !valid {
                                break;
                            }
                            has_exponent = true;
                            num_str.push(ch);
                            self.advance();
                            if let Some(&sign) = self.peek() {
                                if sign == '+' || sign == '-' {
                                    num_str.push(sign);
                                    self.advance();
                                }
                            }
                        } else {
                            break;
                        }
                    }
                    Token::Float(num_str.parse().unwrap_or(0.0))
                } else {
                    Token::Dot
                }
            }
            '|' => {
                if self.peek() == Some(&'|') {
                    self.advance();
                    Token::LogicalOr
                } else if self.peek() == Some(&'=') {
                    self.advance();
                    Token::BitwiseOrAssign
                } else {
                    Token::Pipe
                }
            }
            '&' => {
                if self.peek() == Some(&'&') {
                    self.advance();
                    Token::LogicalAnd
                } else if self.peek() == Some(&'=') {
                    self.advance();
                    Token::BitwiseAndAssign
                } else {
                    Token::Ampersand
                }
            }
            '#' => {
                if self.peek() == Some(&'[') {
                    self.advance();
                    Token::AttributeOpen
                } else {
                    Token::Error("Expected [ after #".to_string())
                }
            }
            '?' => {
                if self.peek() == Some(&'>') {
                    self.advance(); // consume '>'
                    self.in_php_mode = false;
                    Token::CloseTag
                } else if self.peek() == Some(&'-') {
                    self.advance(); // consume '-'
                    if self.peek() == Some(&'>') {
                        self.advance(); // consume '>'
                        Token::NullsafeObjectOperator
                    } else {
                        Token::Error("Expected > after ?-".to_string())
                    }
                } else if self.peek() == Some(&'?') {
                    self.advance(); // consume '?'
                    if self.peek() == Some(&'=') {
                        self.advance(); // consume '='
                        Token::NullCoalesceAssign
                    } else {
                        Token::NullCoalesce
                    }
                } else {
                    Token::QuestionMark
                }
            }
            '(' => Token::OpenParen,
            ')' => Token::CloseParen,
            '{' => Token::OpenBrace,
            '}' => Token::CloseBrace,
            '[' => Token::OpenBracket,
            ']' => Token::CloseBracket,
            ';' => Token::Semicolon,
            ',' => Token::Comma,
            ':' => {
                if self.peek() == Some(&':') {
                    self.advance();
                    Token::DoubleColon
                } else {
                    Token::Colon
                }
            }
            '=' => {
                if self.peek() == Some(&'=') {
                    self.advance(); // consume second '='
                    if self.peek() == Some(&'=') {
                        self.advance(); // consume third '='
                        Token::StrictEquals
                    } else {
                        Token::Equals
                    }
                } else if self.peek() == Some(&'>') {
                    self.advance(); // consume '>'
                    Token::FatArrow
                } else {
                    Token::Assign
                }
            }
            '<' => {
                if self.peek() == Some(&'=') {
                    self.advance();
                    if self.peek() == Some(&'>') {
                        self.advance();
                        Token::Spaceship
                    } else {
                        Token::LessThanOrEqual
                    }
                } else if self.peek() == Some(&'<') {
                    self.advance();
                    if self.peek() == Some(&'=') {
                        self.advance();
                        Token::ShiftLeftAssign
                    } else if self.peek() == Some(&'<') {
                        self.advance();
                        self.parse_heredoc()
                    } else {
                        Token::ShiftLeft
                    }
                } else if self.peek() == Some(&'?') {
                    // OpenTag <? or <?php
                    self.advance(); // consume '?'
                    // check for 'php'
                    let mut p1 = self.chars.clone();
                    if p1.next() == Some('p') && p1.next() == Some('h') && p1.next() == Some('p') {
                        // it's php, let's consume it and maybe trailing whitespace
                        self.advance(); // p
                        self.advance(); // h
                        self.advance(); // p
                        if let Some(&ws) = self.peek() {
                            if ws == ' ' || ws == '\t' || ws == '\n' || ws == '\r' {
                                self.advance();
                            }
                        }
                    }
                    Token::OpenTag
                } else {
                    Token::LessThan
                }
            }
            '>' => {
                if self.peek() == Some(&'=') {
                    self.advance();
                    Token::GreaterThanOrEqual
                } else if self.peek() == Some(&'>') {
                    self.advance();
                    if self.peek() == Some(&'=') {
                        self.advance();
                        Token::ShiftRightAssign
                    } else {
                        Token::ShiftRight
                    }
                } else {
                    Token::GreaterThan
                }
            }
            '!' => {
                if self.peek() == Some(&'=') {
                    self.advance();
                    if self.peek() == Some(&'=') {
                        self.advance();
                        Token::StrictNotEquals
                    } else {
                        Token::NotEquals
                    }
                } else {
                    Token::Not
                }
            }
            '%' => {
                if self.peek() == Some(&'=') {
                    self.advance();
                    Token::ModuloAssign
                } else {
                    Token::Modulo
                }
            }
            '$' => {
                let mut var_name = String::new();
                while let Some(&ch) = self.peek() {
                    if ch.is_alphanumeric() || ch == '_' {
                        var_name.push(ch);
                        self.advance();
                    } else {
                        break;
                    }
                }
                Token::Variable(var_name)
            }
            '"' | '\'' => {
                let quote = c;
                let mut string_val = String::new();
                let mut is_closed = false;
                let mut is_interpolated = false;
                let mut parts = Vec::new();
                
                while let Some(ch) = self.advance() {
                    if ch == quote {
                        is_closed = true;
                        break;
                    }
                    if ch == '\\' {
                        if quote == '"' {
                            // Double-quoted string: handle escape sequences
                            if let Some(escaped) = self.advance() {
                                match escaped {
                                    'n' => string_val.push('\n'),
                                    'r' => string_val.push('\r'),
                                    't' => string_val.push('\t'),
                                    'v' => string_val.push('\x0B'),
                                    'e' => string_val.push('\x1B'),
                                    'f' => string_val.push('\x0C'),
                                    '\\' => string_val.push('\\'),
                                    '"' => string_val.push('"'),
                                    '$' => string_val.push('$'),
                                    'x' => {
                                        let mut hex_str = String::new();
                                        for _ in 0..2 {
                                            if let Some(&hc) = self.peek() {
                                                if hc.is_ascii_hexdigit() {
                                                    hex_str.push(hc);
                                                    self.advance();
                                                } else {
                                                    break;
                                                }
                                            }
                                        }
                                        if !hex_str.is_empty() {
                                            if let Ok(byte_val) = u8::from_str_radix(&hex_str, 16) {
                                                string_val.push(byte_val as char);
                                            }
                                        } else {
                                            string_val.push('\\');
                                            string_val.push('x');
                                        }
                                    }
                                    'u' => {
                                        if self.peek() == Some(&'{') {
                                            self.advance(); // consume '{'
                                            let mut hex_str = String::new();
                                            while let Some(&uc) = self.peek() {
                                                if uc == '}' {
                                                    self.advance(); // consume '}'
                                                    break;
                                                } else if uc.is_ascii_hexdigit() {
                                                    hex_str.push(uc);
                                                    self.advance();
                                                } else {
                                                    break;
                                                }
                                            }
                                            if let Ok(code) = u32::from_str_radix(&hex_str, 16) {
                                                if let Some(ch) = char::from_u32(code) {
                                                    string_val.push(ch);
                                                }
                                            }
                                        } else {
                                            string_val.push('\\');
                                            string_val.push('u');
                                        }
                                    }
                                    '0'..='7' => {
                                        let mut oct_str = String::from(escaped);
                                        for _ in 0..2 {
                                            if let Some(&oc) = self.peek() {
                                                if ('0'..='7').contains(&oc) {
                                                    oct_str.push(oc);
                                                    self.advance();
                                                } else {
                                                    break;
                                                }
                                            }
                                        }
                                        if let Ok(byte_val) = u8::from_str_radix(&oct_str, 8) {
                                            string_val.push(byte_val as char);
                                        }
                                    }
                                    _ => {
                                        // Unknown escape: keep backslash + char
                                        string_val.push('\\');
                                        string_val.push(escaped);
                                    }
                                }
                            }
                        } else {
                            // Single-quoted string: only \\ and \' are special
                            if let Some(escaped) = self.advance() {
                                match escaped {
                                    '\'' => string_val.push('\''),
                                    '\\' => string_val.push('\\'),
                                    _ => {
                                        string_val.push('\\');
                                        string_val.push(escaped);
                                    }
                                }
                            }
                        }
                    } else if quote == '"' && ch == '{' && self.peek() == Some(&'$') {
                        // Complex expression interpolation: {$...}
                        is_interpolated = true;
                        if !string_val.is_empty() {
                            parts.push(InterpolatedPart::Literal(string_val.clone()));
                            string_val.clear();
                        }
                        let mut expr_code = String::new();
                        let mut depth = 1;
                        let mut in_str_quote = None;
                        let mut is_escaped = false;
                        while let Some(&ic) = self.peek() {
                            self.advance();
                            if let Some(sq) = in_str_quote {
                                if is_escaped {
                                    is_escaped = false;
                                } else if ic == '\\' {
                                    is_escaped = true;
                                } else if ic == sq {
                                    in_str_quote = None;
                                }
                                expr_code.push(ic);
                            } else {
                                if ic == '\'' || ic == '"' {
                                    in_str_quote = Some(ic);
                                    expr_code.push(ic);
                                } else if ic == '{' {
                                    depth += 1;
                                    expr_code.push(ic);
                                } else if ic == '}' {
                                    depth -= 1;
                                    if depth == 0 {
                                        break;
                                    }
                                    expr_code.push(ic);
                                } else {
                                    expr_code.push(ic);
                                }
                            }
                        }
                        parts.push(InterpolatedPart::Expression(expr_code));
                    } else if quote == '"' && ch == '$' {
                        let is_var_start = match self.peek() {
                            Some(&vc) => vc.is_ascii_alphabetic() || vc == '_' || vc == '{',
                            None => false,
                        };
                        if !is_var_start {
                            string_val.push('$');
                            continue;
                        }

                        let mut var_expr = String::from("$");
                        let mut var_name = String::new();
                        while let Some(&vc) = self.peek() {
                            if vc.is_ascii_alphanumeric() || vc == '_' {
                                var_name.push(vc);
                                var_expr.push(vc);
                                self.advance();
                            } else {
                                break;
                            }
                        }
                        if var_name.is_empty() {
                            // Just a literal $
                            string_val.push('$');
                        } else {
                            is_interpolated = true;
                            if !string_val.is_empty() {
                                parts.push(InterpolatedPart::Literal(std::mem::take(&mut string_val)));
                            }
                            // Check for $var->prop or $var[index]
                            if self.peek() == Some(&'-') {
                                let mut cloned = self.clone();
                                cloned.advance(); // '-'
                                if cloned.peek() == Some(&'>') {
                                    cloned.advance(); // '>'
                                    if let Some(&first_prop) = cloned.peek() {
                                        if first_prop.is_ascii_alphabetic() || first_prop == '_' {
                                            self.advance(); // '-'
                                            self.advance(); // '>'
                                            var_expr.push_str("->");
                                            while let Some(&pc) = self.peek() {
                                                if pc.is_ascii_alphanumeric() || pc == '_' {
                                                    var_expr.push(pc);
                                                    self.advance();
                                                } else {
                                                    break;
                                                }
                                            }
                                        }
                                    }
                                }
                            } else if self.peek() == Some(&'[') {
                                let mut cloned = self.clone();
                                cloned.advance(); // '['
                                let mut has_close = false;
                                let mut idx_content = String::new();
                                while let Some(&ac) = cloned.peek() {
                                    cloned.advance();
                                    if ac == ']' {
                                        has_close = true;
                                        break;
                                    }
                                    if ac == '"' || ac == '\n' {
                                        break;
                                    }
                                    idx_content.push(ac);
                                }
                                if has_close && !idx_content.is_empty() {
                                    self.advance(); // '['
                                    var_expr.push('[');
                                    while let Some(&ac) = self.peek() {
                                        self.advance();
                                        var_expr.push(ac);
                                        if ac == ']' {
                                            break;
                                        }
                                    }
                                }
                            }
                            parts.push(InterpolatedPart::Expression(var_expr));
                        }
                    } else {
                        string_val.push(ch);
                    }
                }

                if is_closed {
                    let has_exprs = parts.iter().any(|p| matches!(p, InterpolatedPart::Expression(_)));
                    if has_exprs {
                        if !string_val.is_empty() {
                            parts.push(InterpolatedPart::Literal(string_val));
                        }
                        Token::InterpolatedString(parts)
                    } else {
                        let mut full_literal = String::new();
                        for p in parts {
                            if let InterpolatedPart::Literal(l) = p {
                                full_literal.push_str(&l);
                            }
                        }
                        full_literal.push_str(&string_val);
                        Token::StringLiteral(full_literal)
                    }
                } else {
                    Token::Error("Unterminated string literal".to_string())
                }
            }
            '0'..='9' => {
                // Non-decimal bases: 0x/0X hex, 0b/0B binary, 0o/0O octal (PHP 8.1+),
                // and the legacy bare-leading-zero octal form (0777).
                if c == '0' {
                    if let Some(&base_ch) = self.peek() {
                        let radix = match base_ch {
                            'x' | 'X' => Some(16),
                            'b' | 'B' => Some(2),
                            'o' | 'O' => Some(8),
                            _ => None,
                        };
                        if let Some(radix) = radix {
                            self.advance(); // consume the base marker
                            let mut digits = String::new();
                            while let Some(&ch) = self.peek() {
                                if ch == '_' {
                                    self.advance();
                                } else if ch.is_digit(radix) {
                                    digits.push(ch);
                                    self.advance();
                                } else {
                                    break;
                                }
                            }
                            return i64::from_str_radix(&digits, radix)
                                .map(Token::Integer)
                                .unwrap_or_else(|_| {
                                    Token::Error(format!("Invalid base-{} literal", radix))
                                });
                        }
                    }
                }

                let mut num_str = String::from(c);
                let mut is_float = false;
                let mut has_exponent = false;

                while let Some(&ch) = self.peek() {
                    if ch.is_ascii_digit() {
                        num_str.push(ch);
                        self.advance();
                    } else if ch == '_' {
                        self.advance();
                    } else if ch == '.' && !is_float && !has_exponent {
                        // A `.` only continues the number if a digit follows,
                        // so `1..2` and `$x = 1 . "a"` still lex correctly.
                        if self.peek_at(1).is_some_and(|c| c.is_ascii_digit()) {
                            is_float = true;
                            num_str.push(ch);
                            self.advance();
                        } else {
                            break;
                        }
                    } else if (ch == 'e' || ch == 'E') && !has_exponent {
                        let next = self.peek_at(1);
                        let valid = match next {
                            Some(n) if n.is_ascii_digit() => true,
                            Some('+') | Some('-') => {
                                self.peek_at(2).is_some_and(|c| c.is_ascii_digit())
                            }
                            _ => false,
                        };
                        if !valid {
                            break;
                        }
                        has_exponent = true;
                        is_float = true;
                        num_str.push(ch);
                        self.advance();
                        if let Some(&sign) = self.peek() {
                            if sign == '+' || sign == '-' {
                                num_str.push(sign);
                                self.advance();
                            }
                        }
                    } else {
                        break;
                    }
                }

                if is_float {
                    Token::Float(num_str.parse().unwrap_or(0.0))
                } else if num_str.len() > 1 && num_str.starts_with('0') {
                    // Legacy octal: 0777 == 511. Digits 8/9 are invalid here.
                    i64::from_str_radix(&num_str[1..], 8)
                        .map(Token::Integer)
                        .unwrap_or_else(|_| Token::Error(format!("Invalid octal literal {}", num_str)))
                } else {
                    // Integers that overflow i64 become floats, as in PHP.
                    match num_str.parse::<i64>() {
                        Ok(i) => Token::Integer(i),
                        Err(_) => Token::Float(num_str.parse().unwrap_or(0.0)),
                    }
                }
            }
            '@' => Token::At,
            '^' => {
                if self.peek() == Some(&'=') {
                    self.advance();
                    Token::BitwiseXorAssign
                } else {
                    Token::BitwiseXor
                }
            }
            _ if c.is_alphabetic() || c == '_' || c == '\\' => {
                let mut ident = String::from(c);
                while let Some(&ch) = self.peek() {
                    if ch.is_alphanumeric() || ch == '_' || ch == '\\' {
                        ident.push(ch);
                        self.advance();
                    } else {
                        break;
                    }
                }
                
                // Keywords vs Identifiers
                match ident.to_lowercase().as_str() {
                    "if" => Token::If,
                    "elseif" => Token::Elseif,
                    "else" => Token::Else,
                    "echo" => Token::Echo,
                    "function" => Token::Function,
                    "return" => Token::Return,
                    "class" => Token::Class,
                    "new" => Token::New,
                    "while" => Token::While,
                    "for" => Token::For,
                    "fn" => Token::Fn,
                    "foreach" => Token::Foreach,
                    "as" => Token::As,
                    "switch" => Token::Switch,
                    "match" => Token::Match,
                    "case" => Token::Case,
                    "default" => Token::Default,
                    "break" => Token::Break,
                    "continue" => Token::Continue,
                    "use" => Token::Use,
                    "require" => Token::Require,
                    "include" => Token::Include,
                    "require_once" => Token::RequireOnce,
                    "include_once" => Token::IncludeOnce,
                    "try" => Token::Try,
                    "catch" => Token::Catch,
                    "finally" => Token::Finally,
                    "throw" => Token::Throw,
                    "namespace" => Token::Namespace,
                    "trait" => Token::Trait,
                    "interface" => Token::Interface,
                    "implements" => Token::Implements,
                    "public" => Token::Public,
                    "protected" => Token::Protected,
                    "private" => Token::Private,
                    "readonly" => Token::Readonly,
                    "static" => Token::Static,
                    "extends" => Token::Extends,
                    "enum" => Token::Enum,
                    "clone" => Token::Clone,
                    "declare" => Token::Declare,
                    "final" => Token::Final,
                    "abstract" => Token::Abstract,
                    "const" => Token::Const,
                    "yield" => Token::Yield,
                    "self" => Token::Self_,
                    "instanceof" => Token::InstanceOf,
                    "isset" => Token::Isset,
                    "empty" => Token::Empty,
                    "unset" => Token::Unset,
                    "list" => Token::List,
                    "print" => Token::Print,
                    "eval" => Token::Eval,
                    "do" => Token::Do,
                    "global" => Token::Global,
                    "endif" => Token::Endif,
                    "endwhile" => Token::Endwhile,
                    "endfor" => Token::Endfor,
                    "endforeach" => Token::Endforeach,
                    "endswitch" => Token::Endswitch,
                    "and" => Token::LogicalAnd,
                    "or" => Token::LogicalOr,
                    "xor" => Token::BitwiseXor,
                    _ => Token::Identifier(ident),
                }
            }
            _ => Token::Error(format!("Unexpected character: {}", c)),
        }
    }

    fn parse_heredoc(&mut self) -> Token {
        while let Some(&c) = self.peek() {
            if c == ' ' || c == '\t' {
                self.advance();
            } else {
                break;
            }
        }

        let mut is_nowdoc = false;
        if self.peek() == Some(&'\'') {
            is_nowdoc = true;
            self.advance();
        } else if self.peek() == Some(&'"') {
            self.advance();
        }

        let mut identifier = String::new();
        while let Some(&c) = self.peek() {
            if c.is_alphanumeric() || c == '_' {
                identifier.push(c);
                self.advance();
            } else {
                break;
            }
        }

        if is_nowdoc && self.peek() == Some(&'\'') {
            self.advance();
        } else if !is_nowdoc && self.peek() == Some(&'"') {
            self.advance();
        }

        // Consume until end of first line (header)
        while let Some(c) = self.advance() {
            if c == '\n' {
                break;
            }
        }

        let mut content = String::new();
        let mut indent_str = String::new();

        loop {
            // Check if upcoming line is the closing identifier
            let mut cloned = self.chars.clone();
            let mut leading_ws = String::new();
            while let Some(ch) = cloned.next() {
                if ch == ' ' || ch == '\t' {
                    leading_ws.push(ch);
                } else {
                    // check if matches identifier
                    let mut matched = true;
                    if ch != identifier.chars().next().unwrap_or('\0') {
                        matched = false;
                    } else {
                        for expected in identifier.chars().skip(1) {
                            if cloned.next() != Some(expected) {
                                matched = false;
                                break;
                            }
                        }
                    }
                    if matched {
                        // Check following char: cannot be alphanumeric or _
                        let next_ch = cloned.next();
                        let is_end_of_ident = match next_ch {
                            Some(c) if c.is_alphanumeric() || c == '_' => false,
                            _ => true,
                        };
                        if is_end_of_ident {
                            // Match found! Consume leading_ws and identifier from self.chars
                            indent_str = leading_ws;
                            for _ in 0..indent_str.len() + identifier.len() {
                                self.advance();
                            }
                            // Strip trailing newline before closing marker if any
                            if content.ends_with("\r\n") {
                                content.pop();
                                content.pop();
                            } else if content.ends_with('\n') {
                                content.pop();
                            }
                            let final_content = if !indent_str.is_empty() {
                                let mut stripped = String::new();
                                for (i, line) in content.split('\n').enumerate() {
                                    if i > 0 {
                                        stripped.push('\n');
                                    }
                                    if line.starts_with(&indent_str) {
                                        stripped.push_str(&line[indent_str.len()..]);
                                    } else {
                                        stripped.push_str(line);
                                    }
                                }
                                stripped
                            } else {
                                content
                            };

                            if is_nowdoc {
                                return Token::StringLiteral(final_content);
                            } else {
                                return Self::interpolate_heredoc(&final_content);
                            }
                        }
                    }
                    break;
                }
            }

            // Not the closing identifier: consume line up to and including \n
            let mut found_any = false;
            while let Some(c) = self.advance() {
                found_any = true;
                content.push(c);
                if c == '\n' {
                    break;
                }
            }
            if !found_any {
                break;
            }
        }

        if is_nowdoc {
            Token::StringLiteral(content)
        } else {
            Self::interpolate_heredoc(&content)
        }
    }

    fn interpolate_heredoc(input: &str) -> Token {
        let mut chars = input.chars().peekable();
        let mut parts = Vec::new();
        let mut string_val = String::new();
        let mut is_interpolated = false;

        while let Some(ch) = chars.next() {
            if ch == '\\' {
                if let Some(escaped) = chars.next() {
                    match escaped {
                        'n' => string_val.push('\n'),
                        'r' => string_val.push('\r'),
                        't' => string_val.push('\t'),
                        'v' => string_val.push('\x0B'),
                        'e' => string_val.push('\x1B'),
                        'f' => string_val.push('\x0C'),
                        '\\' => string_val.push('\\'),
                        '$' => string_val.push('$'),
                        _ => {
                            string_val.push('\\');
                            string_val.push(escaped);
                        }
                    }
                } else {
                    string_val.push('\\');
                }
            } else if ch == '{' && chars.peek() == Some(&'$') {
                is_interpolated = true;
                if !string_val.is_empty() {
                    parts.push(InterpolatedPart::Literal(std::mem::take(&mut string_val)));
                }
                let mut expr_code = String::new();
                let mut depth = 1;
                let mut in_str_quote = None;
                let mut is_escaped = false;
                while let Some(&ic) = chars.peek() {
                    chars.next();
                    if let Some(sq) = in_str_quote {
                        if is_escaped {
                            is_escaped = false;
                        } else if ic == '\\' {
                            is_escaped = true;
                        } else if ic == sq {
                            in_str_quote = None;
                        }
                        expr_code.push(ic);
                    } else {
                        if ic == '\'' || ic == '"' {
                            in_str_quote = Some(ic);
                            expr_code.push(ic);
                        } else if ic == '{' {
                            depth += 1;
                            expr_code.push(ic);
                        } else if ic == '}' {
                            depth -= 1;
                            if depth == 0 {
                                break;
                            }
                            expr_code.push(ic);
                        } else {
                            expr_code.push(ic);
                        }
                    }
                }
                parts.push(InterpolatedPart::Expression(expr_code));
            } else if ch == '$' {
                let is_var_start = match chars.peek() {
                    Some(&vc) => vc.is_ascii_alphabetic() || vc == '_' || vc == '{',
                    None => false,
                };
                if !is_var_start {
                    string_val.push('$');
                    continue;
                }

                let mut var_expr = String::from("$");
                let mut var_name = String::new();
                while let Some(&vc) = chars.peek() {
                    if vc.is_ascii_alphanumeric() || vc == '_' {
                        var_name.push(vc);
                        var_expr.push(vc);
                        chars.next();
                    } else {
                        break;
                    }
                }
                if var_name.is_empty() {
                    string_val.push('$');
                } else {
                    is_interpolated = true;
                    if !string_val.is_empty() {
                        parts.push(InterpolatedPart::Literal(std::mem::take(&mut string_val)));
                    }
                    // Check for $var->prop or $var[index]
                    if chars.peek() == Some(&'-') {
                        let mut cloned = chars.clone();
                        cloned.next(); // '-'
                        if cloned.peek() == Some(&'>') {
                            cloned.next(); // '>'
                            if let Some(&first_prop_char) = cloned.peek() {
                                if first_prop_char.is_ascii_alphabetic() || first_prop_char == '_' {
                                    chars.next(); // '-'
                                    chars.next(); // '>'
                                    var_expr.push_str("->");
                                    while let Some(&pc) = chars.peek() {
                                        if pc.is_ascii_alphanumeric() || pc == '_' {
                                            var_expr.push(pc);
                                            chars.next();
                                        } else {
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    } else if chars.peek() == Some(&'[') {
                        let mut cloned = chars.clone();
                        cloned.next(); // '['
                        let mut has_close = false;
                        let mut idx_content = String::new();
                        while let Some(&ac) = cloned.peek() {
                            cloned.next();
                            if ac == ']' {
                                has_close = true;
                                break;
                            }
                            if ac == '\n' {
                                break;
                            }
                            idx_content.push(ac);
                        }
                        if has_close && !idx_content.is_empty() {
                            chars.next(); // '['
                            var_expr.push('[');
                            while let Some(ac) = chars.next() {
                                var_expr.push(ac);
                                if ac == ']' {
                                    break;
                                }
                            }
                        }
                    }
                    parts.push(InterpolatedPart::Expression(var_expr));
                }
            } else {
                string_val.push(ch);
            }
        }

        let has_exprs = parts.iter().any(|p| matches!(p, InterpolatedPart::Expression(_)));
        if has_exprs {
            if !string_val.is_empty() {
                parts.push(InterpolatedPart::Literal(string_val));
            }
            Token::InterpolatedString(parts)
        } else {
            let mut full_literal = String::new();
            for p in parts {
                if let InterpolatedPart::Literal(l) = p {
                    full_literal.push_str(&l);
                }
            }
            full_literal.push_str(&string_val);
            Token::StringLiteral(full_literal)
        }
    }
}
// TODO(TechDebt): Refactor Lexer to use zero-copy &'a str instead of String for extreme performance
impl<'a> Iterator for Lexer<'a> {
    type Item = TokenRecord;

    fn next(&mut self) -> Option<Self::Item> {
        let record = self.next_token();
        if record.token == Token::Eof {
            None
        } else {
            Some(record)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lexer_tracking() {
        let source = "<?php
$a = 42;
if ($a === 42) {
    echo \"hello\";
}
";
        let mut lexer = Lexer::new(source);
        let t_open = lexer.next_token();
        assert_eq!(t_open.token, Token::OpenTag);
        
        let t1 = lexer.next_token();
        assert_eq!(t1.token, Token::Variable("a".to_string()));
        assert_eq!(t1.line, 2);
        assert_eq!(t1.column, 1); // $a starts at col 1

        assert_eq!(lexer.next_token().token, Token::Assign);
        assert_eq!(lexer.next_token().token, Token::Integer(42));
        assert_eq!(lexer.next_token().token, Token::Semicolon);
        
        let t_if = lexer.next_token();
        assert_eq!(t_if.token, Token::If);
        assert_eq!(t_if.line, 3);
        assert_eq!(t_if.column, 1);
        
        lexer.next_token(); // (
        lexer.next_token(); // $a
        
        let t_eq = lexer.next_token();
        assert_eq!(t_eq.token, Token::StrictEquals); // ===
        
        lexer.next_token(); // 42
        lexer.next_token(); // )
        lexer.next_token(); // {
        
        let t_echo = lexer.next_token();
        assert_eq!(t_echo.token, Token::Echo);
        
        let t_str = lexer.next_token();
        assert_eq!(t_str.token, Token::StringLiteral("hello".to_string()));
        
        lexer.next_token(); // ;
        lexer.next_token(); // }
        assert_eq!(lexer.next_token().token, Token::Eof);
    }
}
