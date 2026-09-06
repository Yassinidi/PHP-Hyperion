pub mod ast;

use crate::lexer::{Token, TokenRecord};
use ast::{Expr, Stmt, Program};

pub struct Parser<I>
where
    I: Iterator<Item = TokenRecord> + Clone,
{
    tokens: std::iter::Peekable<I>,
    pub errors: usize,
    pub function_depth: usize,
}

impl<I> Parser<I>
where
    I: Iterator<Item = TokenRecord> + Clone,
{
    pub fn new(lexer: I) -> Self {
        Self {
            tokens: lexer.peekable(),
            errors: 0,
            function_depth: 0,
        }
    }

    pub fn parse_attributes(&mut self) -> Vec<crate::ast::Attribute> {
        let mut attrs = Vec::new();
        while self.peek().map(|t| &t.token) == Some(&Token::AttributeOpen) {
            self.advance(); // consume '#['
            while let Some(record) = self.peek() {
                if record.token == Token::CloseBracket || record.token == Token::Eof {
                    break;
                }
                // Parse attribute name
                let mut attr_name = String::new();
                while let Some(p) = self.peek() {
                    if let Some(id) = Self::token_as_identifier(&p.token) {
                        attr_name.push_str(&id);
                        self.advance();
                        continue;
                    }
                    if matches!(&p.token, Token::Identifier(id) if id == "\\\\") {
                        attr_name.push('\\');
                        self.advance();
                        continue;
                    }
                    break;
                }
                if attr_name.is_empty() {
                    break;
                }

                let mut args = Vec::new();
                if self.peek().map(|t| &t.token) == Some(&Token::OpenParen) {
                    self.advance(); // consume '('
                    while let Some(arg_rec) = self.peek() {
                        if arg_rec.token == Token::CloseParen || arg_rec.token == Token::Eof {
                            break;
                        }
                        // Check for named argument: identifier followed by colon
                        if let Some(id) = Self::token_as_identifier(&arg_rec.token) {
                            if self.peek_ahead(1).map(|t| t.token) == Some(Token::Colon) {
                                self.advance(); // identifier
                                self.advance(); // colon
                                if let Some(expr) = self.parse_expression(0) {
                                    args.push(crate::ast::Expr::NamedArgument {
                                        name: id,
                                        value: Box::new(expr),
                                    });
                                }
                                if self.peek().map(|t| &t.token) == Some(&Token::Comma) {
                                    self.advance();
                                }
                                continue;
                            }
                        }

                        if let Some(expr) = self.parse_expression(0) {
                            args.push(expr);
                        }
                        if self.peek().map(|t| &t.token) == Some(&Token::Comma) {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    self.match_token(Token::CloseParen);
                }

                attrs.push(crate::ast::Attribute {
                    name: attr_name,
                    arguments: args,
                });

                if self.peek().map(|t| &t.token) == Some(&Token::Comma) {
                    self.advance();
                } else {
                    break;
                }
            }
            self.match_token(Token::CloseBracket);
        }
        attrs
    }

    fn skip_attributes(&mut self) {
        let _ = self.parse_attributes();
    }

    fn advance(&mut self) -> Option<TokenRecord> {
        self.tokens.next()
    }

    fn token_as_identifier(token: &Token) -> Option<String> {
        match token {
            Token::Identifier(n) => Some(n.clone()),
            Token::Empty => Some("empty".to_string()),
            Token::List => Some("list".to_string()),
            Token::Clone => Some("clone".to_string()),
            Token::Default => Some("default".to_string()),
            Token::Use => Some("use".to_string()),
            Token::Catch => Some("catch".to_string()),
            Token::Throw => Some("throw".to_string()),
            Token::New => Some("new".to_string()),
            Token::Class => Some("class".to_string()),
            Token::Return => Some("return".to_string()),
            Token::Static => Some("static".to_string()),
            Token::Self_ => Some("self".to_string()),
            Token::Public => Some("public".to_string()),
            Token::Protected => Some("protected".to_string()),
            Token::Private => Some("private".to_string()),
            Token::Final => Some("final".to_string()),
            Token::Abstract => Some("abstract".to_string()),
            Token::Fn => Some("fn".to_string()),
            Token::Match => Some("match".to_string()),
            Token::Switch => Some("switch".to_string()),
            Token::For => Some("for".to_string()),
            Token::Foreach => Some("foreach".to_string()),
            Token::While => Some("while".to_string()),
            Token::Do => Some("do".to_string()),
            Token::If => Some("if".to_string()),
            Token::Elseif => Some("elseif".to_string()),
            Token::Else => Some("else".to_string()),
            Token::Echo => Some("echo".to_string()),
            Token::Print => Some("print".to_string()),
            Token::Yield => Some("yield".to_string()),
            Token::Include => Some("include".to_string()),
            Token::IncludeOnce => Some("include_once".to_string()),
            Token::Require => Some("require".to_string()),
            Token::RequireOnce => Some("require_once".to_string()),
            Token::Namespace => Some("namespace".to_string()),
            Token::Trait => Some("trait".to_string()),
            Token::Interface => Some("interface".to_string()),
            Token::Extends => Some("extends".to_string()),
            Token::Implements => Some("implements".to_string()),
            Token::As => Some("as".to_string()),
            Token::InstanceOf => Some("instanceof".to_string()),
            Token::Global => Some("global".to_string()),
            Token::Isset => Some("isset".to_string()),
            Token::Unset => Some("unset".to_string()),
            Token::Eval => Some("eval".to_string()),
            Token::Declare => Some("declare".to_string()),
            Token::Readonly => Some("readonly".to_string()),
            Token::Const => Some("const".to_string()),
            Token::Enum => Some("enum".to_string()),
            Token::Case => Some("case".to_string()),
            Token::Finally => Some("finally".to_string()),
            Token::LogicalAnd => Some("and".to_string()),
            Token::LogicalOr => Some("or".to_string()),
            Token::BitwiseXor => Some("xor".to_string()),
            _ => None,
        }
    }

    pub fn peek(&mut self) -> Option<&TokenRecord> {
        self.tokens.peek()
    }

    /// Route a parsed class-body member into the property or the method bucket.
    ///
    /// Constants ride along with the properties because that is the list the
    /// compiler walks when it builds the class. A `const A = 1, B = 2;` arrives
    /// as a `Block` of declarations, so it is flattened here — pushed whole it
    /// would be mistaken for a method body and never registered.
    fn push_class_member(stmt: Stmt, properties: &mut Vec<Stmt>, methods: &mut Vec<Stmt>) {
        match stmt {
            Stmt::PropertyDeclaration { .. } | Stmt::ConstDeclaration { .. } => {
                properties.push(stmt)
            }
            Stmt::Block(inner)
                if !inner.is_empty()
                    && inner
                        .iter()
                        .all(|s| matches!(s, Stmt::ConstDeclaration { .. })) =>
            {
                properties.extend(inner);
            }
            _ => methods.push(stmt),
        }
    }

    fn parse_trait_adaptations(&mut self, trait_aliases: &mut Vec<(Option<String>, String, String)>) {
        if self.peek().map(|t| &t.token) == Some(&Token::OpenBrace) {
            self.advance(); // consume '{'
            while let Some(peek3) = self.peek() {
                if peek3.token == Token::CloseBrace || peek3.token == Token::Eof {
                    self.advance();
                    break;
                }
                let mut clause_tokens = Vec::new();
                while let Some(clause_peek) = self.peek() {
                    if clause_peek.token == Token::Semicolon {
                        self.advance();
                        break;
                    }
                    if clause_peek.token == Token::CloseBrace || clause_peek.token == Token::Eof {
                        break;
                    }
                    clause_tokens.push(clause_peek.token.clone());
                    self.advance();
                }

                if let Some(as_pos) = clause_tokens.iter().position(|t| matches!(t, Token::As)) {
                    let before = &clause_tokens[..as_pos];
                    let after = &clause_tokens[as_pos + 1..];

                    let mut trait_name = None;
                    let mut orig_method = String::new();

                    if let Some(colon_pos) = before.iter().position(|t| matches!(t, Token::DoubleColon)) {
                        let mut tn = String::new();
                        for t in &before[..colon_pos] {
                            if let Some(id) = Self::token_as_identifier(t) {
                                tn.push_str(&id);
                            } else if t == &Token::Identifier("\\\\".to_string()) {
                                tn.push('\\');
                            }
                        }
                        if !tn.is_empty() {
                            trait_name = Some(tn);
                        }
                        for t in &before[colon_pos + 1..] {
                            if let Some(id) = Self::token_as_identifier(t) {
                                orig_method = id;
                            }
                        }
                    } else {
                        for t in before {
                            if let Some(id) = Self::token_as_identifier(t) {
                                orig_method = id;
                            }
                        }
                    }

                    let mut alias_method = String::new();
                    for t in after {
                        if matches!(t, Token::Public | Token::Protected | Token::Private) {
                            continue;
                        }
                        if let Some(id) = Self::token_as_identifier(t) {
                            alias_method = id;
                        }
                    }

                    if !orig_method.is_empty() && !alias_method.is_empty() {
                        hyperion_core::hyp_debug!("DEBUG PARSER: Found trait alias: {:?}::{} as {}", trait_name, orig_method, alias_method);
                        trait_aliases.push((trait_name, orig_method, alias_method));
                    }
                }
            }
        } else {
            self.match_token(Token::Semicolon);
        }
    }

    /// Look `n` tokens past the current one without consuming anything.
    /// `peek_ahead(0)` is equivalent to `peek()`.
    fn peek_ahead(&mut self, n: usize) -> Option<TokenRecord> {
        self.tokens.clone().nth(n)
    }

    fn match_token(&mut self, expected: Token) -> bool {
        if let Some(record) = self.peek() {
            if record.token == expected {
                self.advance();
                return true;
            }
        }
        false
    }

    fn check_alternative_keyword(&mut self, keywords: &[Token]) -> bool {
        if let Some(record) = self.peek() {
            if keywords.contains(&record.token) {
                return true;
            }
            if record.token == Token::OpenTag {
                if let Some(next) = self.peek_ahead(1) {
                    if keywords.contains(&next.token) {
                        return true;
                    }
                }
            }
        }
        false
    }

    fn match_alternative_keyword(&mut self, keyword: Token) -> bool {
        if let Some(record) = self.peek() {
            if record.token == keyword {
                self.advance();
                return true;
            }
            if record.token == Token::OpenTag {
                if let Some(next) = self.peek_ahead(1) {
                    if next.token == keyword {
                        self.advance(); // consume OpenTag
                        self.advance(); // consume keyword
                        return true;
                    }
                }
            }
        }
        false
    }

    fn skip_return_type_hint(&mut self) {
        if self.peek().map(|t| &t.token) == Some(&Token::Colon) {
            self.advance(); // consume ':'
            // consume type hint
            let mut type_hint_str = String::new();
            while let Some(p) = self.peek() {
                if let Some(id) = Self::token_as_identifier(&p.token) {
                    type_hint_str.push_str(&id);
                    self.advance();
                    continue;
                }
                match &p.token {
                    Token::Pipe | Token::QuestionMark | Token::Ampersand => {
                        self.advance();
                    }
                    _ => break,
                }
            }
        }
    }

    pub fn parse_program(&mut self) -> Program {
        let mut statements = Vec::new();
        while self.peek().is_some() {
            if let Some(record) = self.peek() {
                if record.token == Token::Eof {
                    break;
                }
                if record.token == Token::OpenTag
                    || record.token == Token::CloseTag
                {
                    self.advance();
                    continue;
                }
            }
            if let Some(stmt) = self.parse_statement() {
                statements.push(stmt);
            } else {
                if self.errors == 0 {
                    hyperion_core::hyp_debug!("[DEBUG PARSER] Global statement parse failed. Token: {:?}", self.peek());
                }
                self.errors += 1;
                self.advance();
            }
        }
        statements
    }

    fn parse_arguments(&mut self) -> Vec<Expr> {
        let mut args = Vec::new();
        while let Some(peek_record) = self.peek() {
            if peek_record.token == Token::CloseParen {
                break;
            }
            if peek_record.token == Token::Ellipsis {
                self.advance();
                if self.peek().map(|t| &t.token) == Some(&Token::CloseParen) {
                    args.push(Expr::FirstClassCallable(Box::new(Expr::LiteralNull)));
                    break;
                } else if let Some(unpacked_expr) = self.parse_expression(0) {
                    args.push(Expr::Unpack(Box::new(unpacked_expr)));
                }
                if self.peek().map(|t| &t.token) == Some(&Token::Comma) {
                    self.advance();
                }
                continue;
            }
            if let Some(id) = Self::token_as_identifier(&peek_record.token) {
                if self.peek_ahead(1).map(|t| t.token) == Some(Token::Colon) {
                    self.advance(); // identifier / keyword
                    self.advance(); // ':'
                    if let Some(value) = self.parse_expression(0) {
                        args.push(Expr::NamedArgument {
                            name: id,
                            value: Box::new(value),
                        });
                    }
                    if self.peek().map(|t| &t.token) == Some(&Token::Comma) {
                        self.advance(); // ','
                    }
                    continue;
                }
            }
            if let Some(mut arg_expr) = self.parse_expression(0) {
                if let Expr::Identifier(ref name) = arg_expr {
                    if self.peek().map(|t| &t.token) == Some(&Token::Colon) {
                        self.advance(); // consume ':'
                        if let Some(value) = self.parse_expression(0) {
                            arg_expr = Expr::NamedArgument {
                                name: name.clone(),
                                value: Box::new(value),
                            };
                        }
                    }
                }
                args.push(arg_expr);
            }
            if self.peek().map(|t| &t.token) == Some(&Token::Comma) {
                self.advance(); // consume ','
            }
        }
        self.match_token(Token::CloseParen);
        args
    }

    fn parse_parameters(&mut self) -> Vec<crate::ast::ParamDef> {
        let mut params = Vec::new();

        while let Some(peek_record) = self.peek() {
            if peek_record.token == Token::CloseParen {
                break;
            }

            self.skip_attributes();
            let mut visibility = None;

            // Check for property promotion (public, protected, private, readonly)
            let mut _is_readonly = false;
            while let Some(p) = self.peek() {
                match p.token {
                    Token::Public | Token::Protected | Token::Private => {
                        visibility = match p.token {
                            Token::Public => Some(crate::ast::Visibility::Public),
                            Token::Protected => Some(crate::ast::Visibility::Protected),
                            Token::Private => Some(crate::ast::Visibility::Private),
                            _ => None,
                        };
                        self.advance();
                    }
                    Token::Readonly => {
                        _is_readonly = true;
                        self.advance();
                    }
                    _ => break,
                }
            }

            // Capture type hint
            let mut type_hint_str = String::new();
            while let Some(p) = self.peek() {
                if let Some(id) = Self::token_as_identifier(&p.token) {
                    type_hint_str.push_str(&id);
                    self.advance();
                    continue;
                }
                match &p.token {
                    Token::Identifier(id) if id == "\\\\" => {
                        type_hint_str.push('\\');
                        self.advance();
                    }
                    Token::QuestionMark => {
                        type_hint_str.push('?');
                        self.advance();
                    }
                    Token::Pipe => {
                        type_hint_str.push('|');
                        self.advance();
                    }
                    Token::Ampersand => {
                        // `&` directly before the variable marks a by-reference
                        // parameter; between type names it is an intersection
                        // type (`A&B`). Only the latter belongs in the hint.
                        if matches!(
                            self.peek_ahead(1).map(|t| t.token),
                            Some(Token::Variable(_)) | Some(Token::Ellipsis)
                        ) {
                            break;
                        }
                        type_hint_str.push('&');
                        self.advance();
                    }
                    _ => break,
                }
            }

            // By-reference marker, e.g. `function f(&$x)` or `f(&...$rest)`.
            let mut by_ref = false;
            if self.peek().map(|t| &t.token) == Some(&Token::Ampersand) {
                self.advance();
                by_ref = true;
            }

            let mut is_variadic = false;
            if self.peek().map(|t| &t.token) == Some(&Token::Ellipsis) {
                self.advance();
                is_variadic = true;
            }
            
            if let Some(p) = self.peek() {
                if let Token::Variable(name) = &p.token {
                    let mut param = crate::ast::ParamDef {
                        name: name.clone(),
                        type_hint: if type_hint_str.is_empty() { None } else { Some(type_hint_str) },
                        has_default: false,
                        default_expr: None,
                        is_variadic,
                        by_ref,
                        promoted_visibility: visibility,
                    };
                    self.advance();
                    
                    // Parse default value if present
                    if let Some(peek_assign) = self.peek() {
                        if peek_assign.token == Token::Assign {
                            self.advance(); // consume '='
                            if let Some(default_val) = self.parse_expression(0) {
                                param.has_default = true;
                                param.default_expr = Some(default_val);
                            }
                        }
                    }
                    params.push(param);
                } else {
                    // It could be an error, or maybe we expected a variable but got something else.
                    // Just break to prevent eating the entire file!
                    break;
                }
            }
            
            if self.peek().map(|t| &t.token) == Some(&Token::Comma) {
                self.advance();
            }
        }
        params
    }

    pub fn parse_statement(&mut self) -> Option<Stmt> {
        while let Some(record) = self.peek() {
            if record.token == Token::OpenTag || record.token == Token::CloseTag {
                self.advance();
            } else {
                break;
            }
        }

        let attributes = self.parse_attributes();
        
        if let Some(record) = self.peek() {
            match &record.token {
                Token::InlineHtml(text) => {
                    let text = text.clone();
                    self.advance();
                    return Some(Stmt::Echo(vec![Expr::LiteralString(text)]));
                }
                Token::OpenTag | Token::CloseTag => {
                    self.advance();
                    return self.parse_statement();
                }
                Token::OpenBrace => {
                    self.advance();
                    let mut stmts = Vec::new();
                    while let Some(peek_record) = self.peek() {
                        if peek_record.token == Token::OpenTag || peek_record.token == Token::CloseTag {
                            self.advance();
                            continue;
                        }
                        if peek_record.token == Token::CloseBrace || peek_record.token == Token::Eof {
                            break;
                        }
                        if let Some(stmt) = self.parse_statement() {
                            stmts.push(stmt);
                        } else {
                            self.advance();
                        }
                    }
                    self.match_token(Token::CloseBrace);
                    Some(Stmt::Block(stmts))
                }
                Token::If => {
                    self.advance(); // consume 'if'
                    self.match_token(Token::OpenParen);
                    let condition = self.parse_expression(0)?;
                    self.match_token(Token::CloseParen);
                    
                    let mut body = Vec::new();
                    let mut elseif_branches = Vec::new();
                    let mut else_body = None;
                    // check if alternate syntax
                    if self.peek().map(|t| &t.token) == Some(&Token::Colon) {
                        self.advance(); // consume ':'
                        while let Some(peek_record) = self.peek() {
                            if matches!(peek_record.token, Token::CloseBrace | Token::Eof)
                                || self.check_alternative_keyword(&[Token::Elseif, Token::Else, Token::Endif])
                            {
                                break;
                            }
                            if let Some(stmt) = self.parse_statement() {
                                body.push(stmt);
                            } else {
                                self.advance();
                            }
                        }
                        
                        while self.check_alternative_keyword(&[Token::Elseif]) {
                            self.match_alternative_keyword(Token::Elseif); // consume optional OpenTag + 'elseif'
                            self.match_token(Token::OpenParen);
                            let elseif_cond = self.parse_expression(0)?;
                            self.match_token(Token::CloseParen);
                            self.match_token(Token::Colon);
                            
                            let mut elseif_body = Vec::new();
                            while let Some(peek) = self.peek() {
                                if matches!(peek.token, Token::CloseBrace | Token::Eof)
                                    || self.check_alternative_keyword(&[Token::Elseif, Token::Else, Token::Endif])
                                {
                                    break;
                                }
                                if let Some(stmt) = self.parse_statement() {
                                    elseif_body.push(stmt);
                                } else {
                                    self.advance();
                                }
                            }
                            elseif_branches.push((elseif_cond, elseif_body));
                        }
                        
                        if self.check_alternative_keyword(&[Token::Else]) {
                            self.match_alternative_keyword(Token::Else); // consume optional OpenTag + 'else'
                            self.match_token(Token::Colon);
                            let mut e_body = Vec::new();
                            while let Some(peek) = self.peek() {
                                if matches!(peek.token, Token::CloseBrace | Token::Eof)
                                    || self.check_alternative_keyword(&[Token::Endif])
                                {
                                    break;
                                }
                                if let Some(stmt) = self.parse_statement() {
                                    e_body.push(stmt);
                                } else {
                                    self.advance();
                                }
                            }
                            else_body = Some(e_body);
                        }
                        if self.check_alternative_keyword(&[Token::Endif]) {
                            self.match_alternative_keyword(Token::Endif);
                        } else {
                            self.match_token(Token::CloseBrace);
                        }
                        if self.peek().map(|t| &t.token) == Some(&Token::Semicolon) {
                            self.advance();
                        }
                    } else {
                        // normal syntax
                        if let Some(stmt) = self.parse_statement() {
                            body.push(stmt);
                        }
                        
                        while let Some(peek_record) = self.peek() {
                            if peek_record.token == Token::Elseif {
                                self.advance(); // consume 'elseif'
                                self.match_token(Token::OpenParen);
                                let elseif_cond = self.parse_expression(0)?;
                                self.match_token(Token::CloseParen);
                                let mut elseif_body = Vec::new();
                                if let Some(stmt) = self.parse_statement() {
                                    elseif_body.push(stmt);
                                }
                                elseif_branches.push((elseif_cond, elseif_body));
                            } else {
                                break;
                            }
                        }
                        
                        if let Some(peek_record) = self.peek() {
                            if peek_record.token == Token::Else {
                                self.advance(); // consume 'else'
                                let mut e_body = Vec::new();
                                if let Some(stmt) = self.parse_statement() {
                                    e_body.push(stmt);
                                }
                                else_body = Some(e_body);
                            }
                        }
                    }
                    
                    let mut final_else = else_body;
                    for (elseif_cond, elseif_body) in elseif_branches.into_iter().rev() {
                        final_else = Some(vec![Stmt::If {
                            condition: elseif_cond,
                            then_branch: elseif_body,
                            else_branch: final_else,
                        }]);
                    }
                    Some(Stmt::If { condition, then_branch: body, else_branch: final_else })
                }
                Token::While => {
                    self.advance(); // consume 'while'
                    self.match_token(Token::OpenParen);
                    let condition = self.parse_expression(0)?;
                    self.match_token(Token::CloseParen);
                    
                    let mut body = Vec::new();
                    if self.peek().map(|t| &t.token) == Some(&Token::Colon) {
                        self.advance(); // consume ':'
                        while let Some(peek_record) = self.peek() {
                            if matches!(peek_record.token, Token::CloseBrace | Token::Eof)
                                || self.check_alternative_keyword(&[Token::Endwhile])
                            {
                                break;
                            }
                            if let Some(stmt) = self.parse_statement() {
                                body.push(stmt);
                            } else {
                                self.advance();
                            }
                        }
                        if self.check_alternative_keyword(&[Token::Endwhile]) {
                            self.match_alternative_keyword(Token::Endwhile);
                        } else {
                            self.match_token(Token::CloseBrace);
                        }
                        if self.peek().map(|t| &t.token) == Some(&Token::Semicolon) {
                            self.advance();
                        }
                    } else {
                        if let Some(stmt) = self.parse_statement() {
                            body.push(stmt);
                        }
                    }
                    Some(Stmt::While { condition, body })
                }
                Token::For => {
                    self.advance(); // consume 'for'
                    self.match_token(Token::OpenParen);
                    
                    let mut init = Vec::new();
                    if self.peek().map(|t| &t.token) != Some(&Token::Semicolon) {
                        init.push(self.parse_expression(0)?);
                        while self.peek().map(|t| &t.token) == Some(&Token::Comma) {
                            self.advance();
                            init.push(self.parse_expression(0)?);
                        }
                    }
                    self.match_token(Token::Semicolon);
                    
                    let mut cond = Vec::new();
                    if self.peek().map(|t| &t.token) != Some(&Token::Semicolon) {
                        cond.push(self.parse_expression(0)?);
                        while self.peek().map(|t| &t.token) == Some(&Token::Comma) {
                            self.advance();
                            cond.push(self.parse_expression(0)?);
                        }
                    }
                    self.match_token(Token::Semicolon);
                    
                    let mut step = Vec::new();
                    if self.peek().map(|t| &t.token) != Some(&Token::CloseParen) {
                        step.push(self.parse_expression(0)?);
                        while self.peek().map(|t| &t.token) == Some(&Token::Comma) {
                            self.advance();
                            step.push(self.parse_expression(0)?);
                        }
                    }
                    self.match_token(Token::CloseParen);
                    
                    let mut body = Vec::new();
                    if self.peek().map(|t| &t.token) == Some(&Token::Colon) {
                        self.advance();
                        while let Some(peek_record) = self.peek() {
                            if matches!(peek_record.token, Token::CloseBrace | Token::Eof)
                                || self.check_alternative_keyword(&[Token::Endfor])
                            {
                                break;
                            }
                            if let Some(stmt) = self.parse_statement() {
                                body.push(stmt);
                            } else {
                                self.advance();
                            }
                        }
                        if self.check_alternative_keyword(&[Token::Endfor]) {
                            self.match_alternative_keyword(Token::Endfor);
                        } else {
                            self.match_token(Token::CloseBrace);
                        }
                        if self.peek().map(|t| &t.token) == Some(&Token::Semicolon) {
                            self.advance();
                        }
                    } else if let Some(stmt) = self.parse_statement() {
                        body.push(stmt);
                    }
                    Some(Stmt::For { init, condition: cond, increment: step, body })
                }
                Token::Foreach => {
                    self.advance(); // consume 'foreach'
                    self.match_token(Token::OpenParen);
                    let array = self.parse_expression(0)?;
                    self.match_token(Token::As);
                    
                    let mut key = None;
                    let mut value = self.parse_expression(0)?;
                    
                    if self.peek().map(|t| &t.token) == Some(&Token::FatArrow) {
                        self.advance(); // consume '=>'
                        key = Some(value);
                        value = self.parse_expression(0)?;
                    }
                    self.match_token(Token::CloseParen);
                    
                    let mut body = Vec::new();
                    if self.peek().map(|t| &t.token) == Some(&Token::Colon) {
                        self.advance(); // consume ':'
                        while let Some(peek_record) = self.peek() {
                            if matches!(peek_record.token, Token::CloseBrace | Token::Eof)
                                || self.check_alternative_keyword(&[Token::Endforeach])
                            {
                                break;
                            }
                            if let Some(stmt) = self.parse_statement() {
                                body.push(stmt);
                            } else {
                                self.advance();
                            }
                        }
                        if self.check_alternative_keyword(&[Token::Endforeach]) {
                            self.match_alternative_keyword(Token::Endforeach);
                        } else {
                            self.match_token(Token::CloseBrace);
                        }
                        if self.peek().map(|t| &t.token) == Some(&Token::Semicolon) {
                            self.advance();
                        }
                    } else {
                        if let Some(stmt) = self.parse_statement() {
                            body.push(stmt);
                        }
                    }
                    let key_var = if let Some(crate::parser::ast::Expr::Variable(v)) = key { Some(v) } else { None };
                    // `foreach ($a as &$v)` parses the value as MakeRef; unwrap
                    // it into a flag the compiler can act on.
                    let (value, value_by_ref) = match value {
                        crate::parser::ast::Expr::MakeRef(inner) => (*inner, true),
                        other => (other, false),
                    };
                    let (value_var, body) = match value {
                        crate::parser::ast::Expr::Variable(v) => (v, body),
                        crate::parser::ast::Expr::Array(elements) => {
                            static FOREACH_CTR: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
                            let temp_var = format!("__hyperion_foreach_destruct_{}", FOREACH_CTR.fetch_add(1, std::sync::atomic::Ordering::Relaxed));
                            let mut new_body = vec![
                                Stmt::ExprStmt(Expr::ArrayDestructure {
                                    elements,
                                    value: Box::new(Expr::Variable(temp_var.clone())),
                                })
                            ];
                            new_body.extend(body);
                            (temp_var, new_body)
                        }
                        crate::parser::ast::Expr::List(vars) => {
                            static FOREACH_CTR: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
                            let temp_var = format!("__hyperion_foreach_destruct_{}", FOREACH_CTR.fetch_add(1, std::sync::atomic::Ordering::Relaxed));
                            let mut new_body = vec![
                                Stmt::ExprStmt(Expr::ListDestructure {
                                    vars,
                                    value: Box::new(Expr::Variable(temp_var.clone())),
                                })
                            ];
                            new_body.extend(body);
                            (temp_var, new_body)
                        }
                        _ => ("".to_string(), body),
                    };
                    Some(Stmt::Foreach { iterable: array, key_var, value_var, value_by_ref, body })
                }
                Token::Switch => {
                    self.advance(); // consume 'switch'
                    self.match_token(Token::OpenParen);
                    let condition = self.parse_expression(0)?;
                    self.match_token(Token::CloseParen);
                    
                    let mut cases = Vec::new();
                    let is_colon_syntax = self.peek().map(|t| &t.token) == Some(&Token::Colon);
                    if is_colon_syntax {
                        self.advance();
                    } else {
                        self.match_token(Token::OpenBrace);
                    }
                    
                    while let Some(peek_record) = self.peek() {
                        if is_colon_syntax {
                            if matches!(peek_record.token, Token::Endswitch | Token::CloseBrace | Token::Eof) {
                                break;
                            }
                        } else {
                            if peek_record.token == Token::CloseBrace || peek_record.token == Token::Eof {
                                break;
                            }
                        }
                        
                        if peek_record.token == Token::Case || peek_record.token == Token::Default {
                            let is_default = peek_record.token == Token::Default;
                            self.advance(); // consume 'case' or 'default'
                            
                            let value = if is_default {
                                None
                            } else {
                                Some(self.parse_expression(0)?)
                            };
                            
                            if self.peek().map(|t| &t.token) == Some(&Token::Colon) || self.peek().map(|t| &t.token) == Some(&Token::Semicolon) {
                                self.advance(); // consume ':' or ';'
                            }
                            
                            let mut body = Vec::new();
                            while let Some(inner_peek) = self.peek() {
                                if inner_peek.token == Token::Case || inner_peek.token == Token::Default || inner_peek.token == Token::Endswitch || inner_peek.token == Token::CloseBrace || inner_peek.token == Token::Eof {
                                    break;
                                }
                                if let Some(stmt) = self.parse_statement() {
                                    body.push(stmt);
                                }
                            }
                            cases.push((value, body));
                        } else {
                            // junk inside switch
                            self.advance();
                        }
                    }
                    
                    if is_colon_syntax {
                        if self.peek().map(|t| &t.token) == Some(&Token::Endswitch) {
                            self.advance();
                        } else {
                            self.match_token(Token::CloseBrace);
                        }
                        if self.peek().map(|t| &t.token) == Some(&Token::Semicolon) {
                            self.advance();
                        }
                    } else {
                        self.match_token(Token::CloseBrace);
                    }

                    let mut switch_cases = Vec::new();
                    let mut switch_default = None;
                    for (val_opt, body) in cases {
                        if let Some(val) = val_opt {
                            switch_cases.push((val, body));
                        } else {
                            switch_default = Some(body);
                        }
                    }
                    Some(Stmt::Switch {
                        condition,
                        cases: switch_cases,
                        default: switch_default,
                    })
                }
                Token::Match => {
                    // Match expression as a statement
                    let expr = self.parse_expression(0)?;
                    self.match_token(Token::Semicolon);
                    Some(Stmt::ExprStmt(expr))
                }
                Token::Echo => {
                    self.advance(); // consume 'echo'
                    let mut exprs = Vec::new();
                    exprs.push(self.parse_expression(0)?);
                    while self.peek().map(|t| &t.token) == Some(&Token::Comma) {
                        self.advance();
                        exprs.push(self.parse_expression(0)?);
                    }
                    self.match_token(Token::Semicolon);
                    if exprs.is_empty() { return None; } Some(Stmt::Echo(exprs))
                }
                Token::Return => {
                    self.advance(); // consume 'return'
                    if self.peek().map(|t| &t.token) == Some(&Token::Semicolon) {
                        self.advance();
                        Some(Stmt::Return(Expr::LiteralNull))
                    } else {
                        let expr = self.parse_expression(0)?;
                        self.match_token(Token::Semicolon);
                        Some(Stmt::Return(expr))
                    }
                }
                Token::Break => {
                    self.advance(); // consume 'break'
                    let mut num = None;
                    if let Some(peek) = self.peek() {
                        if let Token::Integer(n) = peek.token {
                            num = Some(n as u32);
                            self.advance();
                        }
                    }
                    self.match_token(Token::Semicolon);
                    Some(Stmt::Break(num))
                }
                Token::Continue => {
                    self.advance(); // consume 'continue'
                    let mut num = None;
                    if let Some(peek) = self.peek() {
                        if let Token::Integer(n) = peek.token {
                            num = Some(n as u32);
                            self.advance();
                        }
                    }
                    self.match_token(Token::Semicolon);
                    Some(Stmt::Continue(num))
                }
                Token::Identifier(id) if id == "continue" => {
                    self.advance(); // consume 'continue'
                    let mut num = None;
                    if let Some(peek) = self.peek() {
                        if let Token::Integer(n) = peek.token {
                            num = Some(n as u32);
                            self.advance();
                        }
                    }
                    self.match_token(Token::Semicolon);
                    Some(Stmt::Continue(num))
                }
                Token::Identifier(id) if id == "global" => {
                    self.advance(); // consume 'global'
                    let mut vars = Vec::new();
                    while let Some(peek) = self.peek() {
                        if let Token::Variable(name) = &peek.token {
                            vars.push(name.clone());
                            self.advance();
                        } else {
                            // could be $$var
                            self.advance();
                        }
                        if self.peek().map(|t| &t.token) == Some(&Token::Comma) {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    self.match_token(Token::Semicolon);
                    Some(Stmt::ExprStmt(Expr::LiteralNull))
                }
                Token::Declare => {
                    self.advance(); // consume 'declare'
                    self.match_token(Token::OpenParen);
                    while let Some(peek) = self.peek() {
                        if peek.token == Token::CloseParen { break; }
                        self.advance();
                    }
                    self.match_token(Token::CloseParen);
                    self.match_token(Token::Semicolon);
                    self.parse_statement()
                }
                Token::Namespace => {
                    self.advance(); // consume 'namespace'
                    let mut name = String::new();
                    while let Some(peek) = self.peek() {
                        if peek.token == Token::Semicolon || peek.token == Token::OpenBrace {
                            break;
                        }
                        if let Token::Identifier(id) = &peek.token {
                            name.push_str(id);
                        } else if peek.token == Token::Identifier("\\".to_string()) {
                            name.push('\\');
                        }
                        self.advance();
                    }
                    
                    if self.peek().map(|t| &t.token) == Some(&Token::Semicolon) {
                        self.advance(); // consume ';'
                    }
                    
                    // Note: if there is an OpenBrace '{', the next call to parse_statement will handle it as a block.
                    Some(Stmt::Namespace(name))
                }
                Token::Use => {
                    self.advance(); // consume 'use'
                    
                    let mut _is_function = false;
                    let mut _is_const = false;
                    
                    if let Some(peek) = self.peek() {
                        if peek.token == Token::Function {
                            _is_function = true;
                            self.advance();
                        } else if peek.token == Token::Const {
                            _is_const = true;
                            self.advance();
                        }
                    }
                    
                    let mut uses = Vec::new();
                    loop {
                        let mut name = String::new();
                        while let Some(peek) = self.peek() {
                            if peek.token == Token::As || peek.token == Token::Comma || peek.token == Token::Semicolon || peek.token == Token::OpenBrace {
                                break;
                            }
                            if let Token::Identifier(id) = &peek.token {
                                name.push_str(id);
                            } else if peek.token == Token::Identifier("\\\\".to_string()) {
                                name.push('\\');
                            }
                            self.advance();
                        }
                        
                        if self.peek().map(|t| &t.token) == Some(&Token::OpenBrace) {
                            // grouped use
                            self.advance();
                            while let Some(peek) = self.peek() {
                                if peek.token == Token::CloseBrace {
                                    break;
                                }
                                let mut sub_name = String::new();
                                while let Some(sub_peek) = self.peek() {
                                    if sub_peek.token == Token::As || sub_peek.token == Token::Comma || sub_peek.token == Token::CloseBrace {
                                        break;
                                    }
                                    if let Some(id) = Self::token_as_identifier(&sub_peek.token) {
                                        sub_name.push_str(&id);
                                    }
                                    self.advance();
                                }
                                
                                let mut alias = None;
                                if self.peek().map(|t| &t.token) == Some(&Token::As) {
                                    self.advance();
                                    if let Some(rec) = self.advance() {
                                        if let Some(id) = Self::token_as_identifier(&rec.token) {
                                            alias = Some(id);
                                        }
                                    }
                                }
                                
                                let full_name = if name.ends_with('\\') {
                                    format!("{}{}", name, sub_name)
                                } else {
                                    format!("{}\\{}", name, sub_name)
                                };
                                uses.push((full_name, alias));
                                
                                if self.peek().map(|t| &t.token) == Some(&Token::Comma) {
                                    self.advance();
                                } else {
                                    break;
                                }
                            }
                            self.match_token(Token::CloseBrace);
                        } else {
                            let mut alias = None;
                            if self.peek().map(|t| &t.token) == Some(&Token::As) {
                                self.advance();
                                if let Some(rec) = self.advance() {
                                    if let Some(id) = Self::token_as_identifier(&rec.token) {
                                        alias = Some(id);
                                    }
                                }
                            }
                            uses.push((name, alias));
                        }
                        
                        if self.peek().map(|t| &t.token) == Some(&Token::Comma) {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    self.match_token(Token::Semicolon);
                    if uses.is_empty() {
                        None
                    } else if uses.len() == 1 {
                        let (class_name, alias) = uses.remove(0);
                        Some(Stmt::Use { class_name, alias })
                    } else {
                        Some(Stmt::Block(
                            uses.into_iter()
                                .map(|(class_name, alias)| Stmt::Use { class_name, alias })
                                .collect(),
                        ))
                    }
                }
                Token::Function => {
                    self.advance(); // consume function
                    if self.peek().map(|t| &t.token) == Some(&Token::Ampersand) {
                        self.advance(); // consume '&'
                    }
                    let name = if let Some(rec) = self.advance() {
                        if let Some(n) = Self::token_as_identifier(&rec.token) { n } else { return None; }
                    } else { return None; };
                    self.match_token(Token::OpenParen);
                    let params = self.parse_parameters();
                    self.match_token(Token::CloseParen);
                    self.skip_return_type_hint();
                    self.match_token(Token::OpenBrace);
                    self.function_depth += 1;
                    let mut body = Vec::new();
                    while let Some(peek_record) = self.peek() {
                        if peek_record.token == Token::OpenTag || peek_record.token == Token::CloseTag {
                            self.advance();
                            continue;
                        }
                        if peek_record.token == Token::CloseBrace || peek_record.token == Token::Eof {
                            break;
                        }
                        if let Some(stmt) = self.parse_statement() {
                            body.push(stmt);
                        } else {
                            self.advance();
                        }
                    }
                    self.function_depth -= 1;
                    self.match_token(Token::CloseBrace);
                    Some(Stmt::Function { name, params, body, is_static: false, visibility: crate::ast::Visibility::Public, attributes })
                }
                Token::Const => {
                    self.advance(); // consume 'const'
                    // `const A = 1, B = 2;` declares both, so the whole list is
                    // collected. Stopping at the first name silently dropped the
                    // rest.
                    let mut decls = Vec::new();
                    loop {
                        let name = match self.advance() {
                            // A constant may be named with a word that is a
                            // keyword elsewhere (`const DEFAULT`, `const LIST`).
                            Some(rec) => match Self::token_as_identifier(&rec.token) {
                                Some(n) => n,
                                None => break,
                            },
                            None => break,
                        };
                        self.match_token(Token::Assign);
                        let Some(value) = self.parse_expression(0) else {
                            break;
                        };
                        decls.push(Stmt::ConstDeclaration {
                            name,
                            value,
                            visibility: crate::ast::Visibility::Public,
                        });
                        if self.peek().map(|t| &t.token) == Some(&Token::Comma) {
                            self.advance();
                            continue;
                        }
                        break;
                    }
                    self.match_token(Token::Semicolon);
                    if decls.len() == 1 {
                        decls.pop()
                    } else {
                        Some(Stmt::Block(decls))
                    }
                }
                Token::Interface => {
                    self.advance(); // consume interface
                    let name = if let Some(rec) = self.advance() {
                        if let Token::Identifier(n) = rec.token { n } else { return None; }
                    } else { return None; };
                    
                    let mut extends = Vec::new();
                    if let Some(peek) = self.peek() {
                        if peek.token == Token::Extends {
                            self.advance(); // consume extends
                            loop {
                                if let Some(rec) = self.advance() {
                                    if let Token::Identifier(n) = rec.token {
                                        extends.push(n);
                                    }
                                }
                                if let Some(peek2) = self.peek() {
                                    if peek2.token == Token::Comma {
                                        self.advance();
                                    } else {
                                        break;
                                    }
                                } else {
                                    break;
                                }
                            }
                        }
                    }
                    
                    self.match_token(Token::OpenBrace);
                    let mut methods = Vec::new();
                    while let Some(peek_record) = self.peek() {
                        if peek_record.token == Token::CloseBrace || peek_record.token == Token::Eof {
                            break;
                        }
                        if let Some(stmt) = self.parse_statement() {
                            methods.push(stmt);
                        }
                    }
                    self.match_token(Token::CloseBrace);
                    Some(Stmt::Interface { name, extends, methods })
                }
                Token::Trait => {
                    self.advance(); // consume trait
                    let name = if let Some(rec) = self.advance() {
                        if let Token::Identifier(n) = rec.token { n } else { return None; }
                    } else { return None; };
                    
                    self.match_token(Token::OpenBrace);
                    let mut methods = Vec::new();
                    let mut properties = Vec::new();
                    let mut uses = Vec::new();
                    let mut trait_aliases = Vec::new();
                    while let Some(peek_record) = self.peek() {
                        if peek_record.token == Token::CloseBrace || peek_record.token == Token::Eof {
                            break;
                        }
                        
                        if peek_record.token == Token::Use {
                            self.advance(); // consume use
                            loop {
                                let mut trait_name = String::new();
                                while let Some(peek) = self.peek() {
                                    if peek.token == Token::As || peek.token == Token::Comma || peek.token == Token::Semicolon || peek.token == Token::OpenBrace {
                                        break;
                                    }
                                    if let Token::Identifier(id) = &peek.token {
                                        trait_name.push_str(id);
                                    } else if peek.token == Token::Identifier("\\\\".to_string()) {
                                        trait_name.push('\\');
                                    }
                                    self.advance();
                                }
                                if !trait_name.is_empty() {
                                    hyperion_core::hyp_debug!("DEBUG PARSER: Found trait use: {} inside trait {}", trait_name, name);
                                    uses.push(trait_name);
                                }
                                if let Some(peek2) = self.peek() {
                                    if peek2.token == Token::Comma {
                                        self.advance();
                                    } else {
                                        break;
                                    }
                                } else {
                                    break;
                                }
                            }
                            self.parse_trait_adaptations(&mut trait_aliases);
                            continue;
                        }
                        
                        if let Some(stmt) = self.parse_statement() {
                            Self::push_class_member(stmt, &mut properties, &mut methods);
                        } else {
                            hyperion_core::hyp_debug!("[DEBUG PARSER] Trait statement parse failed. Token: {:?}", self.peek());
                            self.advance();
                        }
                    }
                    self.match_token(Token::CloseBrace);
                    hyperion_core::hyp_debug!("DEBUG PARSER: Parsed trait {} with {} methods", name, methods.len());
                    Some(Stmt::Trait { name, uses, trait_aliases, methods, properties })
                }
                Token::Enum => {
                    self.advance(); // consume enum
                    let name = if let Some(rec) = self.advance() {
                        if let Token::Identifier(n) = rec.token { n } else { return None; }
                    } else { return None; };
                    
                    let mut backed_type = None;
                    if let Some(peek) = self.peek() {
                        if peek.token == Token::Colon {
                            self.advance(); // consume colon
                            if let Some(rec) = self.advance() {
                                if let Token::Identifier(n) = rec.token {
                                    backed_type = Some(n);
                                }
                            }
                        }
                    }
                    
                    let mut implements = Vec::new();
                    if let Some(peek) = self.peek() {
                        if peek.token == Token::Implements {
                            self.advance(); // consume implements
                            loop {
                                if let Some(rec) = self.advance() {
                                    if let Token::Identifier(n) = rec.token {
                                        implements.push(n);
                                    }
                                }
                                if self.match_token(Token::Comma) {
                                    continue;
                                }
                                break;
                            }
                        }
                    }
                    
                    self.match_token(Token::OpenBrace);
                    let mut cases = Vec::new();
                    let mut methods = Vec::new();
                    while let Some(peek_record) = self.peek() {
                        if peek_record.token == Token::CloseBrace || peek_record.token == Token::Eof {
                            break;
                        }
                        if peek_record.token == Token::Case {
                            self.advance(); // consume case
                            let case_name = if let Some(rec) = self.advance() {
                                if let Token::Identifier(n) = rec.token { n } else { return None; }
                            } else { return None; };
                            
                            let mut value = None;
                            if let Some(peek) = self.peek() {
                                if peek.token == Token::Assign {
                                    self.advance(); // consume assign
                                    value = Some(self.parse_expression(0)?);
                                }
                            }
                            self.match_token(Token::Semicolon);
                            cases.push((case_name, value));
                        } else {
                            if let Some(stmt) = self.parse_statement() {
                                methods.push(stmt);
                            }
                        }
                    }
                    self.match_token(Token::CloseBrace);
                    Some(Stmt::Enum { name, backed_type, implements, cases, methods })
                }
                Token::Class | Token::Final | Token::Abstract | Token::Public | Token::Protected | Token::Private | Token::Readonly | Token::Static => {
                    let mut is_final = false;
                    let mut is_abstract = false;
                    let mut is_static = false;
                    let mut is_readonly = false;
                    let mut has_explicit_visibility = false;
                    let mut visibility = crate::ast::Visibility::Public;
                    
                    loop {
                        if let Some(peek) = self.peek() {
                            match peek.token {
                                Token::Final => { is_final = true; self.advance(); }
                                Token::Abstract => { is_abstract = true; self.advance(); }
                                Token::Static => { is_static = true; self.advance(); }
                                Token::Readonly => { is_readonly = true; self.advance(); }
                                Token::Public => { has_explicit_visibility = true; visibility = crate::ast::Visibility::Public; self.advance(); }
                                Token::Protected => { has_explicit_visibility = true; visibility = crate::ast::Visibility::Protected; self.advance(); }
                                Token::Private => { has_explicit_visibility = true; visibility = crate::ast::Visibility::Private; self.advance(); }
                                _ => break,
                            }
                        } else {
                            break;
                        }
                    }
                    
                    if is_static && !has_explicit_visibility && !is_final && !is_abstract && !is_readonly && self.function_depth > 0 {
                        if let Some(peek) = self.peek() {
                            if let Token::Variable(_) = &peek.token {
                                let mut vars = Vec::new();
                                loop {
                                    if let Some(peek) = self.peek() {
                                        if let Token::Variable(v) = &peek.token {
                                            let var_name = v.clone();
                                            self.advance();
                                            let init_expr = if self.peek().map(|t| &t.token) == Some(&Token::Assign) {
                                                self.advance();
                                                self.parse_expression(0)
                                            } else {
                                                None
                                            };
                                            vars.push((var_name, init_expr));
                                        } else {
                                            break;
                                        }
                                        if self.peek().map(|t| &t.token) == Some(&Token::Comma) {
                                            self.advance();
                                        } else {
                                            break;
                                        }
                                    } else {
                                        break;
                                    }
                                }
                                self.match_token(Token::Semicolon);
                                return Some(Stmt::StaticVar(vars));
                            }
                        }
                    }
                    
                    if let Some(peek) = self.peek() {
                        if peek.token == Token::Class {
                            self.advance(); // consume class
                            let name = if let Some(rec) = self.advance() {
                                if let Token::Identifier(n) = rec.token { n } else { return None; }
                            } else { return None; };
                            
                            let mut extends = None;
                            if let Some(peek2) = self.peek() {
                                if peek2.token == Token::Extends {
                                    self.advance(); // consume extends
                                    if let Some(rec) = self.advance() {
                                        if let Token::Identifier(n) = rec.token {
                                            extends = Some(n);
                                        } else { return None; }
                                    } else { return None; }
                                }
                            }

                            let mut implements = Vec::new();
                            if let Some(peek2) = self.peek() {
                                if peek2.token == Token::Implements {
                                    self.advance(); // consume implements
                                    loop {
                                        if let Some(rec) = self.advance() {
                                            if let Token::Identifier(n) = rec.token {
                                                implements.push(n);
                                            }
                                        }
                                        if let Some(peek3) = self.peek() {
                                            if peek3.token == Token::Comma {
                                                self.advance();
                                            } else { break; }
                                        } else { break; }
                                    }
                                }
                            }
                            
                            self.match_token(Token::OpenBrace);
                            let mut methods = Vec::new();
                            let mut properties = Vec::new();
                            let mut uses = Vec::new();
                            let mut trait_aliases = Vec::new();
                            while let Some(peek_record) = self.peek() {
                                if peek_record.token == Token::OpenTag || peek_record.token == Token::CloseTag {
                                    self.advance();
                                    continue;
                                }
                                if peek_record.token == Token::CloseBrace || peek_record.token == Token::Eof {
                                    break;
                                }
                                
                                if peek_record.token == Token::Use {
                                    self.advance(); // consume use
                                    loop {
                                        let mut trait_name = String::new();
                                        while let Some(peek) = self.peek() {
                                            if peek.token == Token::As || peek.token == Token::Comma || peek.token == Token::Semicolon || peek.token == Token::OpenBrace {
                                                break;
                                            }
                                            if let Token::Identifier(id) = &peek.token {
                                                trait_name.push_str(id);
                                            } else if peek.token == Token::Identifier("\\\\".to_string()) {
                                                trait_name.push('\\');
                                            }
                                            self.advance();
                                        }
                                        if !trait_name.is_empty() {
                                            uses.push(trait_name);
                                        }
                                        if let Some(peek2) = self.peek() {
                                            if peek2.token == Token::Comma {
                                                self.advance();
                                            } else {
                                                break;
                                            }
                                        } else {
                                            break;
                                        }
                                    }
                                    self.parse_trait_adaptations(&mut trait_aliases);
                                    continue;
                                }
                                if let Some(stmt) = self.parse_statement() {
                                    Self::push_class_member(stmt, &mut properties, &mut methods);
                                } else {
                                    self.advance();
                                }
                            }
                            self.match_token(Token::CloseBrace);
                            if name == "Mbstring" {
                                hyperion_core::hyp_debug!("DEBUG PARSER: Mbstring class finished parsing. methods: {:?}", methods.iter().filter_map(|m| if let Stmt::Function { name, .. } = m { Some(name.clone()) } else { None }).collect::<Vec<_>>());
                            }
                            return Some(Stmt::Class { name, is_abstract, is_final, is_readonly, extends, methods, properties, implements, uses, trait_aliases, attributes });
                        } else if peek.token == Token::Function {
                            self.advance(); // consume function
                            if self.peek().map(|t| &t.token) == Some(&Token::Ampersand) {
                                self.advance(); // consume '&'
                            }
                            let name = if let Some(rec) = self.advance() {
                                if let Some(n) = Self::token_as_identifier(&rec.token) { n } else { return None; }
                            } else { return None; };
                            self.match_token(Token::OpenParen);
                            let params = self.parse_parameters();
                            self.match_token(Token::CloseParen);
                            self.skip_return_type_hint();
                            let mut body = Vec::new();
                            if self.peek().map(|t| &t.token) == Some(&Token::Semicolon) {
                                self.advance(); // consume ';'
                            } else {
                                self.match_token(Token::OpenBrace);
                                self.function_depth += 1;
                                while let Some(peek_record) = self.peek() {
                                    if peek_record.token == Token::OpenTag || peek_record.token == Token::CloseTag {
                                        self.advance();
                                        continue;
                                    }
                                    if peek_record.token == Token::CloseBrace || peek_record.token == Token::Eof {
                                        break;
                                    }
                                    if let Some(stmt) = self.parse_statement() {
                                        body.push(stmt);
                                    } else {
                                        self.advance();
                                    }
                                }
                                self.function_depth -= 1;
                                self.match_token(Token::CloseBrace);
                            }
                            if name == "mb_internal_encoding" || name == "mb_convert_encoding" {
                                hyperion_core::hyp_debug!("DEBUG PARSER: parsed method {}", name);
                            }
                            return Some(Stmt::Function { name, params, body, is_static, visibility, attributes });
                        } else if peek.token == Token::DoubleColon {
                            let expr = self.parse_expression_with_left(crate::ast::Expr::Identifier("static".to_string()), 0)?;
                            self.match_token(Token::Semicolon);
                            return Some(Stmt::ExprStmt(expr));
                        } else {
                            // parse property
                            let mut is_const = false;
                            if self.peek().map(|t| &t.token) == Some(&Token::Const) {
                                is_const = true;
                                self.advance();
                            }
                            let mut prop_name = String::new();
                            let mut found_var = false;
                            while let Some(p) = self.peek() {
                                match &p.token {
                                    Token::Variable(n) => {
                                        prop_name = n.clone();
                                        found_var = true;
                                        self.advance();
                                        break;
                                    }
                                    Token::Identifier(n) if is_const => {
                                        prop_name = n.clone();
                                        found_var = true;
                                        self.advance();
                                        break;
                                    }
                                    tok if is_const && Self::token_as_identifier(tok).is_some() => {
                                        prop_name = Self::token_as_identifier(tok).unwrap();
                                        found_var = true;
                                        self.advance();
                                        break;
                                    }
                                    Token::Semicolon | Token::OpenBrace | Token::Function | Token::Eof => break,
                                    _ => { self.advance(); }
                                }
                            }
                            
                            if !found_var {
                                let expr = self.parse_expression_with_left(crate::ast::Expr::Identifier("static".to_string()), 0)?;
                                self.match_token(Token::Semicolon);
                                return Some(Stmt::ExprStmt(expr));
                            }
                            
                            let mut initial_value = None;
                            if let Some(p) = self.peek() {
                                if p.token == Token::Assign {
                                    self.advance();
                                    initial_value = self.parse_expression(0);
                                }
                            }

                            if is_const {
                                // A class constant, not a property. The modifier
                                // path is shared, but the two live in different
                                // tables and `Foo::NAME` reads only the constant
                                // one — which is why `self::COLORS` came back
                                // null while the const sat in instance
                                // properties. `static` and `readonly` carry no
                                // meaning on a constant, so only visibility is
                                // kept.
                                if initial_value.is_none() {
                                    // A typed constant (`const string FOO = 'x'`)
                                    // put the type where the name was expected;
                                    // the real name is the next word.
                                    if let Some(rec) = self.peek() {
                                        if let Some(n) = Self::token_as_identifier(&rec.token) {
                                            prop_name = n;
                                            self.advance();
                                            if self.peek().map(|t| &t.token) == Some(&Token::Assign) {
                                                self.advance();
                                                initial_value = self.parse_expression(0);
                                            }
                                        }
                                    }
                                }
                                let mut decls = vec![Stmt::ConstDeclaration {
                                    name: prop_name,
                                    value: initial_value.unwrap_or(crate::ast::Expr::LiteralNull),
                                    visibility,
                                }];
                                while self.peek().map(|t| &t.token) == Some(&Token::Comma) {
                                    self.advance();
                                    let Some(rec) = self.advance() else { break };
                                    let Some(n) = Self::token_as_identifier(&rec.token) else {
                                        break;
                                    };
                                    self.match_token(Token::Assign);
                                    let Some(v) = self.parse_expression(0) else { break };
                                    decls.push(Stmt::ConstDeclaration {
                                        name: n,
                                        value: v,
                                        visibility,
                                    });
                                }
                                self.match_token(Token::Semicolon);
                                return Some(if decls.len() == 1 {
                                    decls.pop().unwrap()
                                } else {
                                    Stmt::Block(decls)
                                });
                            }

                            if self.peek().map(|t| &t.token) == Some(&Token::OpenBrace) {
                                self.match_token(Token::OpenBrace);
                                let mut hook_depth = 1;
                                while let Some(p) = self.peek() {
                                    if p.token == Token::OpenBrace {
                                        hook_depth += 1;
                                    } else if p.token == Token::CloseBrace {
                                        hook_depth -= 1;
                                        if hook_depth == 0 {
                                            self.advance();
                                            break;
                                        }
                                    } else if p.token == Token::Eof {
                                        break;
                                    }
                                    self.advance();
                                }
                                return Some(Stmt::PropertyDeclaration { name: prop_name, initial_value, is_static, is_readonly, visibility });
                            }
                            
                            loop {
                                if let Some(p) = self.peek() {
                                    match p.token {
                                        Token::Comma => {
                                            self.advance();
                                            while let Some(p2) = self.peek() {
                                                match &p2.token {
                                                    Token::Variable(_) => { self.advance(); break; }
                                                    Token::Semicolon | Token::Eof => break,
                                                    _ => { self.advance(); }
                                                }
                                            }
                                            if let Some(p2) = self.peek() {
                                                if p2.token == Token::Assign {
                                                    self.advance();
                                                    self.parse_expression(0);
                                                }
                                            }
                                        }
                                        Token::Semicolon | Token::Eof => break,
                                        _ => { self.advance(); }
                                    }
                                } else {
                                    break;
                                }
                            }
                            
                            self.match_token(Token::Semicolon);
                            return Some(Stmt::PropertyDeclaration { name: prop_name, initial_value, is_static, is_readonly, visibility });
                        }
                    }
                    None
                }
Token::Try => {
                    self.advance(); // consume 'try'
                    
                    self.match_token(Token::OpenBrace);
                    let mut try_body = Vec::new();
                    while let Some(peek_record) = self.peek() {
                        if peek_record.token == Token::CloseBrace || peek_record.token == Token::Eof {
                            break;
                        }
                        if let Some(stmt) = self.parse_statement() {
                            try_body.push(stmt);
                        }
                    }
                    self.match_token(Token::CloseBrace);
                    
                    let mut catches = Vec::new();
                    while self.peek().map(|t| &t.token) == Some(&Token::Catch) {
                        self.advance(); // consume 'catch'
                        self.match_token(Token::OpenParen);
                        
                        let mut catch_types = Vec::new();
                        while let Some(rec) = self.peek() {
                            if let Token::Identifier(n) = &rec.token {
                                catch_types.push(n.clone());
                                self.advance();
                                if let Some(peek2) = self.peek() {
                                    if peek2.token == Token::Pipe {
                                        self.advance();
                                        continue;
                                    }
                                }
                            }
                            break;
                        }
                        if catch_types.is_empty() {
                            catch_types.push("Exception".to_string());
                        }
                        
                        let mut catch_var = None;
                        if let Some(rec) = self.peek() {
                            if let Token::Variable(v) = &rec.token {
                                catch_var = Some(v.clone());
                                self.advance();
                            }
                        }
                        
                        self.match_token(Token::CloseParen);
                        
                        self.match_token(Token::OpenBrace);
                        let mut catch_body = Vec::new();
                        while let Some(peek_record) = self.peek() {
                            if peek_record.token == Token::CloseBrace || peek_record.token == Token::Eof {
                                break;
                            }
                            if let Some(stmt) = self.parse_statement() {
                                catch_body.push(stmt);
                            }
                        }
                        self.match_token(Token::CloseBrace);
                        catches.push(crate::ast::CatchBlock {
                            types: catch_types,
                            var: catch_var,
                            body: catch_body,
                        });
                    }

                    let mut finally_body = None;
                    if self.peek().map(|t| &t.token) == Some(&Token::Finally) {
                        self.advance(); // consume 'finally'
                        self.match_token(Token::OpenBrace);
                        let mut body = Vec::new();
                        while let Some(peek_record) = self.peek() {
                            if peek_record.token == Token::CloseBrace || peek_record.token == Token::Eof {
                                break;
                            }
                            if let Some(stmt) = self.parse_statement() {
                                body.push(stmt);
                            }
                        }
                        self.match_token(Token::CloseBrace);
                        finally_body = Some(body);
                    }
                    
                    Some(Stmt::TryCatch { try_body, catches, finally_body })
                }
                Token::Throw => {
                    self.advance(); // consume 'throw'
                    let expr = self.parse_expression(0)?;
                    self.match_token(Token::Semicolon);
                    Some(Stmt::Throw(expr))
                }
                Token::Do => {
                    self.advance(); // consume 'do'
                    let mut body = Vec::new();
                    if self.peek().map(|t| &t.token) == Some(&Token::OpenBrace) {
                        self.advance(); // consume '{'
                        while let Some(peek_record) = self.peek() {
                            if peek_record.token == Token::CloseBrace || peek_record.token == Token::Eof {
                                break;
                            }
                            if let Some(stmt) = self.parse_statement() {
                                body.push(stmt);
                            }
                        }
                        self.match_token(Token::CloseBrace);
                    } else {
                        if let Some(stmt) = self.parse_statement() {
                            body.push(stmt);
                        }
                    }
                    self.match_token(Token::While);
                    self.match_token(Token::OpenParen);
                    let condition = self.parse_expression(0)?;
                    self.match_token(Token::CloseParen);
                    self.match_token(Token::Semicolon);
                    Some(Stmt::DoWhile { body, condition })
                }
                Token::Global => {
                    self.advance(); // consume 'global'
                    let mut vars = Vec::new();
                    loop {
                        if let Some(peek) = self.peek() {
                            if let Token::Variable(v) = &peek.token {
                                vars.push(v.clone());
                                self.advance();
                            } else {
                                break;
                            }
                            if self.peek().map(|t| &t.token) == Some(&Token::Comma) {
                                self.advance();
                            } else {
                                break;
                            }
                        } else {
                            break;
                        }
                    }
                    self.match_token(Token::Semicolon);
                    Some(Stmt::GlobalVar(vars))
                }
                Token::Unset => {
                    self.advance(); // consume 'unset'
                    self.match_token(Token::OpenParen);
                    let mut vars = Vec::new();
                    loop {
                        if let Some(expr) = self.parse_expression(0) {
                            vars.push(expr);
                        }
                        if self.peek().map(|t| &t.token) == Some(&Token::Comma) {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    self.match_token(Token::CloseParen);
                    self.match_token(Token::Semicolon);
                    Some(Stmt::Unset(vars))
                }
                _ => {
                    let expr = self.parse_expression(0)?;
                    self.match_token(Token::Semicolon);
                    Some(Stmt::ExprStmt(expr))
                }
            }
        } else {
            None
        }
    }

    fn get_precedence(token: &Token) -> u8 {
        match token {
            Token::Assign | Token::DotAssign | Token::PlusAssign | Token::MinusAssign | Token::MultiplyAssign | Token::DivideAssign | Token::ModuloAssign | Token::BitwiseAndAssign | Token::BitwiseOrAssign | Token::BitwiseXorAssign | Token::ShiftLeftAssign | Token::ShiftRightAssign | Token::PowerAssign | Token::NullCoalesceAssign => 2,
            Token::QuestionMark => 3,
            Token::NullCoalesce => 4,
            Token::LogicalOr => 5,
            Token::LogicalAnd => 6,
            Token::Pipe => 7,
            Token::BitwiseXor => 8,
            Token::Ampersand => 9,
            Token::NotEquals | Token::StrictNotEquals | Token::Equals | Token::StrictEquals => 10,
            Token::LessThan | Token::GreaterThan | Token::LessThanOrEqual | Token::GreaterThanOrEqual | Token::Spaceship => 12,
            Token::ShiftLeft | Token::ShiftRight => 14,
            Token::Plus | Token::Minus | Token::Dot => 20,
            Token::Multiply | Token::Divide | Token::Modulo => 30,
            // PHP binds instanceof tighter than `!` but looser than `**`:
            // `! $x instanceof Foo` parses as `!($x instanceof Foo)`.
            Token::InstanceOf => 33,
            Token::Power => 35,
            Token::PlusPlus | Token::MinusMinus => 45,
            Token::ObjectOperator | Token::NullsafeObjectOperator | Token::DoubleColon => 40,
            Token::OpenParen | Token::OpenBracket => 50,
            _ => 0,
        }
    }

    fn parse_expression(&mut self, precedence: u8) -> Option<Expr> {
        let left = self.parse_prefix()?;
        self.parse_expression_with_left(left, precedence)
    }

    fn parse_expression_with_left(&mut self, mut left: Expr, precedence: u8) -> Option<Expr> {
        while let Some(record) = self.peek() {
            let next_prec = Self::get_precedence(&record.token);
            let is_assign = matches!(
                record.token,
                Token::Assign
                    | Token::DotAssign
                    | Token::PlusAssign
                    | Token::MinusAssign
                    | Token::MultiplyAssign
                    | Token::DivideAssign
                    | Token::ModuloAssign
                    | Token::BitwiseAndAssign
                    | Token::BitwiseOrAssign
                    | Token::BitwiseXorAssign
                    | Token::ShiftLeftAssign
                    | Token::ShiftRightAssign
                    | Token::PowerAssign
                    | Token::NullCoalesceAssign
            );
            let is_assignable = matches!(left, Expr::Variable(_) | Expr::VariableVariable(_) | Expr::PropertyGet { .. } | Expr::StaticPropertyGet { .. } | Expr::ArrayGet { .. } | Expr::Array(_) | Expr::List(_));
            if next_prec == 0 || (!is_assign && next_prec < precedence) || (is_assign && !is_assignable) {
                break;
            }


            let op_token = self.advance().unwrap().token;

            if op_token == Token::ObjectOperator || op_token == Token::NullsafeObjectOperator {
                // -> or ?-> can be followed by:
                // 1. { (for dynamic property: $obj->{$prop})
                // 2. Variable (for dynamic property: $obj->$prop)
                // 3. Identifier (for static property/method: $obj->prop)
                let next_token = self.peek().map(|t| &t.token);
                if next_token == Some(&Token::OpenBrace) {
                    self.advance(); // consume '{'
                    let prop_expr = self.parse_expression(0)?;
                    self.match_token(Token::CloseBrace);
                    if self.peek().map(|t| &t.token) == Some(&Token::OpenParen) {
                        self.advance(); // consume '('
                        let mut args = self.parse_arguments();
                        let is_first_class = args.len() == 1 && matches!(args[0], Expr::FirstClassCallable(ref inner) if matches!(**inner, Expr::LiteralNull));
                        if is_first_class {
                            args.clear();
                        }
                        if op_token == Token::ObjectOperator {
                            left = Expr::DynamicMethodCall {
                                object: Box::new(left),
                                method: Box::new(prop_expr),
                                arguments: args,
                            };
                        } else {
                            left = Expr::DynamicNullsafeMethodCall {
                                object: Box::new(left),
                                method: Box::new(prop_expr),
                                arguments: args,
                            };
                        }
                        if is_first_class {
                            left = Expr::FirstClassCallable(Box::new(left));
                        }
                    } else if op_token == Token::ObjectOperator {
                        left = Expr::PropertyGet {
                            object: Box::new(left),
                            property: Box::new(prop_expr),
                        };
                    } else {
                        left = Expr::NullsafePropertyGet {
                            object: Box::new(left),
                            property: Box::new(prop_expr),
                        };
                    }
                } else if let Some(Token::Variable(_)) = next_token {
                    let var_rec = self.advance().unwrap();
                    let prop_expr = Expr::Variable(match var_rec.token {
                        Token::Variable(n) => n,
                        _ => unreachable!(),
                    });
                    if self.peek().map(|t| &t.token) == Some(&Token::OpenParen) {
                        self.advance(); // consume '('
                        let mut args = self.parse_arguments();
                        let is_first_class = args.len() == 1 && matches!(args[0], Expr::FirstClassCallable(ref inner) if matches!(**inner, Expr::LiteralNull));
                        if is_first_class {
                            args.clear();
                        }
                        if op_token == Token::ObjectOperator {
                            left = Expr::DynamicMethodCall {
                                object: Box::new(left),
                                method: Box::new(prop_expr),
                                arguments: args,
                            };
                        } else {
                            left = Expr::DynamicNullsafeMethodCall {
                                object: Box::new(left),
                                method: Box::new(prop_expr),
                                arguments: args,
                            };
                        }
                        if is_first_class {
                            left = Expr::FirstClassCallable(Box::new(left));
                        }
                    } else if op_token == Token::ObjectOperator {
                        left = Expr::PropertyGet {
                            object: Box::new(left),
                            property: Box::new(prop_expr),
                        };
                    } else {
                        left = Expr::NullsafePropertyGet {
                            object: Box::new(left),
                            property: Box::new(prop_expr),
                        };
                    }
                } else {
                    let ident_token = self.advance().unwrap().token;
                    let name_opt = Self::token_as_identifier(&ident_token);
                    
                    if let Some(name) = name_opt {
                        // Check if it's a method call: $obj->method(...) or $obj?->method(...)
                        if self.peek().map(|t| &t.token) == Some(&Token::OpenParen) {
                            self.advance(); // consume '('
                            let mut args = self.parse_arguments();
                            let is_first_class = args.len() == 1 && matches!(args[0], Expr::FirstClassCallable(ref inner) if matches!(**inner, Expr::LiteralNull));
                            if is_first_class {
                                args.clear();
                            }
                            if op_token == Token::ObjectOperator {
                                left = Expr::MethodCall {
                                    object: Box::new(left),
                                    method: name,
                                    arguments: args,
                                };
                            } else {
                                left = Expr::NullsafeMethodCall {
                                    object: Box::new(left),
                                    method: name,
                                    arguments: args,
                                };
                            }
                            if is_first_class {
                                left = Expr::FirstClassCallable(Box::new(left));
                            }
                        } else {
                            // Property access: $obj->property or $obj?->property
                            let prop_expr = Expr::Identifier(name);
                            if op_token == Token::ObjectOperator {
                                left = Expr::PropertyGet {
                                    object: Box::new(left),
                                    property: Box::new(prop_expr),
                                };
                            } else {
                                left = Expr::NullsafePropertyGet {
                                    object: Box::new(left),
                                    property: Box::new(prop_expr),
                                };
                            }
                        }
                    } else {
                        return None; // Expected identifier after -> or ?->
                    }
                }
            } else if op_token == Token::DoubleColon {
                // :: must be followed by an identifier or variable (e.g. static::$prop or static::method())
                let next_token = self.peek().map(|t| &t.token).cloned();
                
                let (is_variable_member, name) = if next_token == Some(Token::OpenBrace) {
                    self.advance(); // consume '{'
                    let _dynamic_expr = self.parse_expression(0)?;
                    self.match_token(Token::CloseBrace);
                    (false, "{dynamic}".to_string())
                } else {
                    let token = self.advance().unwrap().token;
                    if let Token::Variable(n) = token {
                        (true, n)
                    } else if let Some(n) = Self::token_as_identifier(&token) {
                        (false, n)
                    } else {
                        return None;
                    }
                };
                
                let (class_name_str, is_dynamic_class) = match &left {
                    Expr::Identifier(n) => (n.clone(), false),
                    Expr::LiteralString(n) => (n.clone(), false),
                    Expr::Variable(n) => (format!("${}", n), true),
                    _ => (String::new(), true),
                };
                
                if self.peek().map(|t| &t.token) == Some(&Token::OpenParen) {
                    self.advance(); // consume '('
                    let mut args = self.parse_arguments();
                    let is_first_class = args.len() == 1 && matches!(args[0], Expr::FirstClassCallable(ref inner) if matches!(**inner, Expr::LiteralNull));
                    if is_first_class {
                        args.clear();
                    }
                    if is_variable_member {
                        let class_expr = if is_dynamic_class {
                            Box::new(left)
                        } else {
                            Box::new(Expr::LiteralString(class_name_str))
                        };
                        left = Expr::DynamicStaticMethodCall {
                            class_name: class_expr,
                            method: Box::new(Expr::Variable(name)),
                            arguments: args,
                        };
                    } else if is_dynamic_class {
                        left = Expr::DynamicStaticMethodCall {
                            class_name: Box::new(left),
                            method: Box::new(Expr::LiteralString(name)),
                            arguments: args,
                        };
                    } else {
                        left = Expr::StaticMethodCall {
                            class_name: class_name_str,
                            method: name,
                            arguments: args,
                        };
                    }
                    if is_first_class {
                        left = Expr::FirstClassCallable(Box::new(left));
                    }
                } else if is_variable_member {
                    left = Expr::StaticPropertyGet {
                        class_name: class_name_str,
                        property: name,
                    };
                } else if is_dynamic_class && name.eq_ignore_ascii_case("class") {
                    left = Expr::Call {
                        callee: Box::new(Expr::Identifier("get_class".to_string())),
                        arguments: vec![left],
                    };
                } else if is_dynamic_class {
                    let class_name_expr = Expr::Ternary {
                        condition: Box::new(Expr::Call {
                            callee: Box::new(Expr::Identifier("is_object".to_string())),
                            arguments: vec![left.clone()],
                        }),
                        true_expr: Box::new(Expr::Call {
                            callee: Box::new(Expr::Identifier("get_class".to_string())),
                            arguments: vec![left.clone()],
                        }),
                        false_expr: Box::new(left),
                    };
                    let full_const_name = Expr::BinaryOp {
                        operator: Token::Dot,
                        left: Box::new(class_name_expr),
                        right: Box::new(Expr::LiteralString(format!("::{}", name))),
                    };
                    left = Expr::Call {
                        callee: Box::new(Expr::Identifier("constant".to_string())),
                        arguments: vec![full_const_name],
                    };
                } else {
                    left = Expr::ClassConstFetch {
                        class_name: class_name_str,
                        constant_name: name,
                    };
                }
            } else if op_token == Token::OpenParen {
                let mut args = self.parse_arguments();
                let is_first_class = args.len() == 1 && matches!(args[0], Expr::FirstClassCallable(ref inner) if matches!(**inner, Expr::LiteralNull));
                if is_first_class {
                    args.clear();
                }
                left = Expr::Call {
                    callee: Box::new(left),
                    arguments: args,
                };
                if is_first_class {
                    left = Expr::FirstClassCallable(Box::new(left));
                }
            } else if op_token == Token::OpenBracket {
                let key = if self.peek().map(|t| &t.token) == Some(&Token::CloseBracket) {
                    None
                } else {
                    Some(Box::new(self.parse_expression(0)?))
                };
                self.match_token(Token::CloseBracket);
                left = Expr::ArrayGet {
                    array: Box::new(left),
                    key,
                };
            } else if matches!(op_token, Token::Assign | Token::PlusAssign | Token::MinusAssign | Token::MultiplyAssign | Token::DivideAssign | Token::ModuloAssign | Token::DotAssign | Token::BitwiseAndAssign | Token::BitwiseOrAssign | Token::BitwiseXorAssign | Token::ShiftLeftAssign | Token::ShiftRightAssign | Token::PowerAssign | Token::NullCoalesceAssign) {
                let parsed_right = self.parse_expression(next_prec - 1)?;
                let right = match op_token {
                    Token::PlusAssign => Expr::CompoundAssign { target: Box::new(left.clone()), operator: Token::PlusAssign, value: Box::new(parsed_right) },
                    Token::MinusAssign => Expr::CompoundAssign { target: Box::new(left.clone()), operator: Token::MinusAssign, value: Box::new(parsed_right) },
                    Token::MultiplyAssign => Expr::CompoundAssign { target: Box::new(left.clone()), operator: Token::MultiplyAssign, value: Box::new(parsed_right) },
                    Token::DivideAssign => Expr::CompoundAssign { target: Box::new(left.clone()), operator: Token::DivideAssign, value: Box::new(parsed_right) },
                    Token::ModuloAssign => Expr::CompoundAssign { target: Box::new(left.clone()), operator: Token::ModuloAssign, value: Box::new(parsed_right) },
                    Token::DotAssign => Expr::CompoundAssign { target: Box::new(left.clone()), operator: Token::DotAssign, value: Box::new(parsed_right) },
                    Token::BitwiseAndAssign => Expr::CompoundAssign { target: Box::new(left.clone()), operator: Token::BitwiseAndAssign, value: Box::new(parsed_right) },
                    Token::BitwiseOrAssign => Expr::CompoundAssign { target: Box::new(left.clone()), operator: Token::BitwiseOrAssign, value: Box::new(parsed_right) },
                    Token::BitwiseXorAssign => Expr::CompoundAssign { target: Box::new(left.clone()), operator: Token::BitwiseXorAssign, value: Box::new(parsed_right) },
                    Token::ShiftLeftAssign => Expr::CompoundAssign { target: Box::new(left.clone()), operator: Token::ShiftLeftAssign, value: Box::new(parsed_right) },
                    Token::ShiftRightAssign => Expr::CompoundAssign { target: Box::new(left.clone()), operator: Token::ShiftRightAssign, value: Box::new(parsed_right) },
                    Token::PowerAssign => Expr::CompoundAssign { target: Box::new(left.clone()), operator: Token::PowerAssign, value: Box::new(parsed_right) },
                    Token::NullCoalesceAssign => Expr::CompoundAssign { target: Box::new(left.clone()), operator: Token::NullCoalesceAssign, value: Box::new(parsed_right) },
                    _ => parsed_right,
                };
                match left {
                    Expr::Variable(target) => {
                        left = Expr::Assignment {
                            target,
                            value: Box::new(right),
                        };
                    }
                    Expr::VariableVariable(target) => {
                        left = Expr::VariableVariableAssign {
                            target,
                            value: Box::new(right),
                        };
                    }
                    Expr::PropertyGet { object, property } => {
                        left = Expr::PropertySet {
                            object,
                            property,
                            value: Box::new(right),
                        };
                    }
                    Expr::StaticPropertyGet { class_name, property } => {
                        left = Expr::StaticPropertySet {
                            class_name,
                            property,
                            value: Box::new(right),
                        };
                    }
                    Expr::ArrayGet { array, key } => {
                        left = Expr::ArraySet {
                            array,
                            key,
                            value: Box::new(right),
                        };
                    }
                    Expr::Array(elements) => {
                        left = Expr::ArrayDestructure {
                            elements,
                            value: Box::new(right),
                        };
                    }
                    Expr::List(vars) => {
                        left = Expr::ListDestructure {
                            vars,
                            value: Box::new(right),
                        };
                    }
                    _ => return None, // Invalid assignment target
                }
            } else if op_token == Token::QuestionMark {
                if self.peek().map(|t| &t.token) == Some(&Token::Colon) {
                    self.advance(); // consume ':'
                    let false_expr = self.parse_expression(2)?;
                    left = Expr::Elvis {
                        condition: Box::new(left),
                        false_expr: Box::new(false_expr),
                    };
                } else {
                    // Ternary operator
                    let true_expr = self.parse_expression(0)?;
                    self.match_token(Token::Colon);
                    let false_expr = self.parse_expression(2)?;
                    left = Expr::Ternary {
                        condition: Box::new(left),
                        true_expr: Box::new(true_expr),
                        false_expr: Box::new(false_expr),
                    };
                }
            } else if op_token == Token::PlusPlus {
                left = Expr::PostIncrement(Box::new(left));
            } else if op_token == Token::MinusMinus {
                left = Expr::PostDecrement(Box::new(left));
            } else {
                // Standard binary operator
                let right_prec = if op_token == Token::Power {
                    next_prec
                } else {
                    next_prec + 1
                };
                let right = self.parse_expression(right_prec)?;
                if op_token == Token::NullCoalesce {
                    left = Expr::NullCoalesce {
                        left: Box::new(left),
                        right: Box::new(right),
                    };
                } else {
                    left = Expr::BinaryOp {
                        left: Box::new(left),
                        operator: op_token,
                        right: Box::new(right),
                    };
                }
            }
        }

        Some(left)
    }

    fn parse_prefix(&mut self) -> Option<Expr> {
        if let Some(peek) = self.peek() {
            if matches!(peek.token, Token::CloseBrace | Token::CloseParen | Token::CloseBracket | Token::Semicolon | Token::Eof) {
                return None;
            }
        }
        let record = self.advance()?;
        match &record.token {
            Token::Ampersand => {
                // Prefix `&` takes a reference to an lvalue: `$a = &$b`,
                // `$cur = &$arr['k']`, `f(&$x)`. It binds tighter than every
                // binary operator but *looser* than the postfix accessors, so
                // `&$arr['k']` references the element and not the array —
                // hence 39, just under `->` / `::` / `[` / `(`.
                let inner = self.parse_expression(39)?;
                Some(Expr::MakeRef(Box::new(inner)))
            }
            Token::Integer(i) => Some(Expr::LiteralInt(*i)),
            Token::Float(f) => Some(Expr::LiteralFloat(*f)),
            Token::StringLiteral(s) => Some(Expr::LiteralString(s.to_string())),
            Token::InterpolatedString(parts) => {
                let mut exprs = Vec::new();
                for part in parts {
                    match part {
                        crate::lexer::InterpolatedPart::Literal(s) => exprs.push(Expr::LiteralString(s.clone())),
                        crate::lexer::InterpolatedPart::Variable(v) => exprs.push(Expr::Variable(v.clone())),
                        crate::lexer::InterpolatedPart::Expression(code) => {
                            let mut sub_lexer = crate::lexer::Lexer::new(code);
                            sub_lexer.in_php_mode = true;
                            let mut sub_parser = Parser::new(sub_lexer);
                            if let Some(expr) = sub_parser.parse_expression(0) {
                                exprs.push(expr);
                            } else {
                                exprs.push(Expr::LiteralString(code.clone()));
                            }
                        }
                    }
                }
                Some(Expr::InterpolatedString(exprs))
            },
            Token::Variable(name) => {
                if name.is_empty() {
                    if let Some(inner) = self.parse_prefix() {
                        return Some(Expr::VariableVariable(Box::new(inner)));
                    } else {
                        return None;
                    }
                }
                Some(Expr::Variable(name.to_string()))
            },
            Token::Identifier(name) => {
                let name_lower = name.to_lowercase();
                if name_lower == "array" && self.peek().map(|t| &t.token) == Some(&Token::OpenParen) {
                    self.advance(); // consume '('
                    let mut elements = Vec::new();
                    while let Some(peek_record) = self.peek() {
                        if peek_record.token == Token::CloseParen {
                            break;
                        }
                        if peek_record.token == Token::Comma {
                            self.advance();
                            elements.push((None, None));
                            continue;
                        }
                        if let Some(expr) = self.parse_expression(0) {
                            if self.match_token(Token::FatArrow) {
                                if let Some(val) = self.parse_expression(0) {
                                    elements.push((Some(expr), Some(val)));
                                } else {
                                    return None;
                                }
                            } else {
                                elements.push((None, Some(expr)));
                            }
                        }
                        if self.peek().map(|t| &t.token) == Some(&Token::Comma) {
                            self.advance();
                        }
                    }
                    if !self.match_token(Token::CloseParen) {
                        return None;
                    }
                    Some(Expr::Array(elements))
                } else if name_lower == "null" {
                    Some(Expr::LiteralNull)
                } else if name_lower == "true" {
                    Some(Expr::LiteralBool(true))
                } else if name_lower == "false" {
                    Some(Expr::LiteralBool(false))
                } else {
                    Some(Expr::Identifier(name.to_string()))
                }
            }
            Token::Self_ => Some(Expr::Identifier("self".to_string())),
            Token::Static => {
                let peek_token = self.peek().map(|t| &t.token);
                if peek_token == Some(&Token::Function) {
                    self.advance(); // consume Token::Static
                    self.advance(); // consume Token::Function
                    self.match_token(Token::OpenParen);
                    let params = self.parse_parameters();
                    self.match_token(Token::CloseParen);
                    
                    let mut uses = Vec::new();
                    if self.peek().map(|t| &t.token) == Some(&Token::Use) {
                        self.advance();
                        self.match_token(Token::OpenParen);
                        while let Some(p) = self.peek() {
                            if p.token == Token::CloseParen {
                                break;
                            }
                            if let Token::Ampersand = &p.token {
                                self.advance(); // consume '&'
                                if let Some(next_p) = self.peek() {
                                    if let Token::Variable(name) = &next_p.token {
                                        uses.push((name.clone(), true));
                                        self.advance();
                                    }
                                }
                            } else if let Token::Variable(name) = &p.token {
                                uses.push((name.clone(), false));
                                self.advance();
                            }
                            if self.peek().map(|t| &t.token) == Some(&Token::Comma) {
                                self.advance();
                            } else {
                                break;
                            }
                        }
                        self.match_token(Token::CloseParen);
                    }
                    self.skip_return_type_hint();
                    self.match_token(Token::OpenBrace);
                    self.function_depth += 1;
                    let mut body = Vec::new();
                    while let Some(peek) = self.peek() {
                        if peek.token == Token::CloseBrace || peek.token == Token::Eof {
                            break;
                        }
                        if let Some(stmt) = self.parse_statement() {
                            body.push(stmt);
                        }
                    }
                    self.function_depth -= 1;
                    self.match_token(Token::CloseBrace);
                    Some(Expr::Closure { params, uses, body })
                } else if peek_token == Some(&Token::Fn) {
                    self.advance(); // consume Token::Static
                    self.advance(); // consume Token::Fn
                    self.match_token(Token::OpenParen);
                    let params = self.parse_parameters();
                    self.match_token(Token::CloseParen);
                    self.skip_return_type_hint();
                    if self.peek().map(|t| &t.token) == Some(&Token::FatArrow) {
                        self.advance();
                        let body = self.parse_expression(0)?;
                        Some(Expr::ArrowFunction { params, body: Box::new(body) })
                    } else {
                        None
                    }
                } else {
                    Some(Expr::Identifier("static".to_string()))
                }
            }
            Token::New => {
                let name_token = self.advance().unwrap().token;
                if name_token == Token::Class {
                    let mut arguments = Vec::new();
                    if self.match_token(Token::OpenParen) {
                        arguments = self.parse_arguments();
                    }
                    
                    let mut extends = None;
                    if let Some(peek) = self.peek() {
                        if peek.token == Token::Extends {
                            self.advance(); // consume extends
                            if let Some(rec) = self.advance() {
                                if let Token::Identifier(n) = rec.token {
                                    extends = Some(n);
                                } else {
                                    return None;
                                }
                            } else {
                                return None;
                            }
                        }
                    }
                    
                    let mut implements = Vec::new();
                    if let Some(peek) = self.peek() {
                        if peek.token == Token::Implements {
                            self.advance(); // consume implements
                            loop {
                                if let Some(rec) = self.advance() {
                                    if let Token::Identifier(n) = rec.token {
                                        implements.push(n);
                                    }
                                }
                                if let Some(peek2) = self.peek() {
                                    if peek2.token == Token::Comma {
                                        self.advance();
                                    } else {
                                        break;
                                    }
                                } else {
                                    break;
                                }
                            }
                        }
                    }
                    
                    self.match_token(Token::OpenBrace);
                    let mut methods = Vec::new();
                    let mut properties = Vec::new();
                    let mut uses = Vec::new();
                    while let Some(peek_record) = self.peek() {
                        if peek_record.token == Token::CloseBrace || peek_record.token == Token::Eof {
                            break;
                        }
                        
                        if peek_record.token == Token::Use {
                            self.advance(); // consume use
                            loop {
                                let mut trait_name = String::new();
                                while let Some(peek) = self.peek() {
                                    if peek.token == Token::As || peek.token == Token::Comma || peek.token == Token::Semicolon || peek.token == Token::OpenBrace {
                                        break;
                                    }
                                    if let Token::Identifier(id) = &peek.token {
                                        trait_name.push_str(id);
                                    } else if peek.token == Token::Identifier("\\\\".to_string()) {
                                        trait_name.push('\\');
                                    }
                                    self.advance();
                                }
                                if !trait_name.is_empty() {
                                    uses.push(trait_name);
                                }
                                if let Some(peek2) = self.peek() {
                                    if peek2.token == Token::Comma {
                                        self.advance();
                                    } else {
                                        break;
                                    }
                                } else {
                                    break;
                                }
                            }
                            if self.peek().map(|t| &t.token) == Some(&Token::OpenBrace) {
                                self.advance(); // consume '{'
                                let mut brace_depth = 1;
                                while let Some(peek3) = self.peek() {
                                    if peek3.token == Token::OpenBrace {
                                        brace_depth += 1;
                                        self.advance();
                                    } else if peek3.token == Token::CloseBrace {
                                        brace_depth -= 1;
                                        self.advance();
                                        if brace_depth == 0 {
                                            break;
                                        }
                                    } else if peek3.token == Token::Eof {
                                        break;
                                    } else {
                                        self.advance();
                                    }
                                }
                            } else {
                                self.match_token(Token::Semicolon);
                            }
                            continue;
                        }
                        
                        if let Some(stmt) = self.parse_statement() {
                            Self::push_class_member(stmt, &mut properties, &mut methods);
                        }
                    }
                    self.match_token(Token::CloseBrace);
                    Some(Expr::NewAnonymousClass {
                        extends,
                        implements,
                        uses,
                        methods,
                        properties,
                        arguments,
                    })
                } else {
                    let mut is_dynamic_expr = false;
                    let mut target_expr: Option<Expr> = None;
                    let is_variable_name = matches!(name_token, Token::Variable(_));
                    let mut class_name = match &name_token {
                        Token::Identifier(n) => n.clone(),
                        Token::Self_ => "self".to_string(),
                        Token::Static => "static".to_string(),
                        Token::Variable(v) => {
                            is_dynamic_expr = true;
                            target_expr = Some(Expr::Variable(v.clone()));
                            format!("${}", v)
                        }
                        Token::OpenParen => {
                            let inner_expr = self.parse_expression(0)?;
                            self.match_token(Token::CloseParen);
                            is_dynamic_expr = true;
                            target_expr = Some(inner_expr);
                            "{dynamic}".to_string()
                        }
                        _ => return None,
                    };

                    if self.match_token(Token::DoubleColon) {
                        if let Some(next_tok) = self.advance() {
                            match next_tok.token {
                                Token::Variable(v) => {
                                    is_dynamic_expr = true;
                                    let lhs = if class_name == "static" {
                                        Expr::LateStaticPropertyGet { property: v.clone() }
                                    } else {
                                        Expr::StaticPropertyGet { class_name: class_name.clone(), property: v.clone() }
                                    };
                                    target_expr = Some(lhs);
                                    class_name = format!("{}::${}", class_name, v);
                                }
                                Token::Identifier(id) => {
                                    is_dynamic_expr = true;
                                    let lhs = if is_variable_name && id.eq_ignore_ascii_case("class") {
                                        Expr::Call {
                                            callee: Box::new(Expr::Identifier("get_class".to_string())),
                                            arguments: vec![target_expr.clone().unwrap()],
                                        }
                                    } else {
                                        Expr::ClassConstFetch { class_name: class_name.clone(), constant_name: id.clone() }
                                    };
                                    target_expr = Some(lhs);
                                    class_name = format!("{}::{}", class_name, id);
                                }
                                _ => return None,
                            }
                        }
                    }

                    loop {
                        if let Some(tok) = self.peek() {
                            match &tok.token {
                                Token::ObjectOperator => {
                                    self.advance();
                                    if let Some(prop_tok) = self.advance() {
                                        let prop_expr = match prop_tok.token {
                                            Token::Identifier(id) => Expr::Identifier(id),
                                            Token::Variable(v) => Expr::Variable(v),
                                            Token::OpenBrace => {
                                                let inner = self.parse_expression(0);
                                                self.match_token(Token::CloseBrace);
                                                inner.unwrap_or(Expr::Identifier(String::new()))
                                            }
                                            _ => break,
                                        };
                                        is_dynamic_expr = true;
                                        let base = target_expr.take().unwrap_or_else(|| Expr::Identifier(class_name.clone()));
                                        target_expr = Some(Expr::PropertyGet {
                                            object: Box::new(base),
                                            property: Box::new(prop_expr),
                                        });
                                    } else {
                                        break;
                                    }
                                }
                                Token::NullsafeObjectOperator => {
                                    self.advance();
                                    if let Some(prop_tok) = self.advance() {
                                        let prop_expr = match prop_tok.token {
                                            Token::Identifier(id) => Expr::Identifier(id),
                                            Token::Variable(v) => Expr::Variable(v),
                                            Token::OpenBrace => {
                                                let inner = self.parse_expression(0);
                                                self.match_token(Token::CloseBrace);
                                                inner.unwrap_or(Expr::Identifier(String::new()))
                                            }
                                            _ => break,
                                        };
                                        is_dynamic_expr = true;
                                        let base = target_expr.take().unwrap_or_else(|| Expr::Identifier(class_name.clone()));
                                        target_expr = Some(Expr::NullsafePropertyGet {
                                            object: Box::new(base),
                                            property: Box::new(prop_expr),
                                        });
                                    } else {
                                        break;
                                    }
                                }
                                Token::OpenBracket => {
                                    self.advance();
                                    let key = self.parse_expression(0);
                                    self.match_token(Token::CloseBracket);
                                    is_dynamic_expr = true;
                                    let base = target_expr.take().unwrap_or_else(|| Expr::Identifier(class_name.clone()));
                                    target_expr = Some(Expr::ArrayGet {
                                        array: Box::new(base),
                                        key: key.map(Box::new),
                                    });
                                }
                                _ => break,
                            }
                        } else {
                            break;
                        }
                    }

                    let mut arguments = Vec::new();
                    if self.match_token(Token::OpenParen) {
                        arguments = self.parse_arguments();
                    }

                    if is_dynamic_expr {
                        if let Some(expr) = target_expr {
                            Some(Expr::NewDynamic { class_expr: Box::new(expr), arguments })
                        } else {
                            Some(Expr::New { class_name, arguments })
                        }
                    } else {
                        Some(Expr::New { class_name, arguments })
                    }
                }
            }
            Token::OpenParen => {
                // Could be a type cast: (int) $x
                let mut is_cast = false;
                let mut cast_type = None;
                
                // Let's check if the next tokens are Identifier(type) / Unset and CloseParen
                let mut peek_iter = self.tokens.clone();
                if let Some(t1) = peek_iter.next() {
                    match &t1.token {
                        Token::Identifier(id) => {
                            if let Some(t2) = peek_iter.next() {
                                if t2.token == Token::CloseParen {
                                    is_cast = true;
                                    cast_type = Some(id.to_lowercase());
                                }
                            }
                        }
                        Token::Unset => {
                            if let Some(t2) = peek_iter.next() {
                                if t2.token == Token::CloseParen {
                                    is_cast = true;
                                    cast_type = Some("unset".to_string());
                                }
                            }
                        }
                        _ => {}
                    }
                }
                
                if is_cast {
                    self.advance(); // consume type identifier / keyword
                    self.match_token(Token::CloseParen);
                    
                    let expr = self.parse_expression(20)?; // High precedence for prefix
                    
                    match cast_type.as_deref() {
                        Some("int" | "integer") => Some(Expr::CastInt(Box::new(expr))),
                        Some("float" | "double" | "real") => Some(Expr::CastFloat(Box::new(expr))),
                        Some("string") => Some(Expr::CastString(Box::new(expr))),
                        Some("bool" | "boolean") => Some(Expr::CastBool(Box::new(expr))),
                        Some("array") => Some(Expr::CastArray(Box::new(expr))),
                        Some("object") => Some(Expr::CastObject(Box::new(expr))),
                        Some("unset") => Some(Expr::LiteralNull),
                        _ => Some(Expr::CastInt(Box::new(expr))),
                    }
                } else {
                    let expr = self.parse_expression(0)?;
                    self.match_token(Token::CloseParen);
                    Some(expr)
                }
            }
            Token::Require => {
                let expr = self.parse_expression(0)?;
                Some(Expr::Require(Box::new(expr)))
            }
            Token::Include => {
                let expr = self.parse_expression(0)?;
                Some(Expr::Include(Box::new(expr)))
            }
            Token::RequireOnce => {
                let expr = self.parse_expression(0)?;
                Some(Expr::RequireOnce(Box::new(expr)))
            }
            Token::IncludeOnce => {
                let expr = self.parse_expression(0)?;
                Some(Expr::IncludeOnce(Box::new(expr)))
            }
            Token::Ellipsis => {
                let expr = self.parse_expression(0)?;
                Some(Expr::Unpack(Box::new(expr)))
            }
            Token::Yield => {
                if let Some(peek_rec) = self.peek() {
                    if let Token::Identifier(ref s) = peek_rec.token {
                        if s.eq_ignore_ascii_case("from") {
                            self.advance(); // consume 'from'
                            let expr = self.parse_expression(0)?;
                            return Some(Expr::YieldFrom(Box::new(expr)));
                        }
                    }
                }
                let is_empty = if let Some(peek_rec) = self.peek() {
                    peek_rec.token == Token::Semicolon || peek_rec.token == Token::CloseParen || peek_rec.token == Token::CloseBracket || peek_rec.token == Token::Comma || peek_rec.token == Token::Eof
                } else {
                    true
                };
                if is_empty {
                    Some(Expr::Yield { key: None, value: None })
                } else {
                    let first_expr = self.parse_expression(0)?;
                    if self.match_token(Token::FatArrow) {
                        let value_expr = self.parse_expression(0)?;
                        Some(Expr::Yield {
                            key: Some(Box::new(first_expr)),
                            value: Some(Box::new(value_expr)),
                        })
                    } else {
                        Some(Expr::Yield {
                            key: None,
                            value: Some(Box::new(first_expr)),
                        })
                    }
                }
            }
            Token::Match => {
                self.match_token(Token::OpenParen);
                let subject = self.parse_expression(0)?;
                self.match_token(Token::CloseParen);
                self.match_token(Token::OpenBrace);
                
                let mut arms = Vec::new();
                let mut default_arm = None;
                
                while let Some(peek) = self.peek() {
                    if peek.token == Token::CloseBrace {
                        break;
                    }
                    
                    if peek.token == Token::Default {
                        self.advance(); // consume default
                        if self.peek().map(|t| &t.token) == Some(&Token::FatArrow) {
                            self.advance(); // =>
                        }
                        default_arm = Some(Box::new(self.parse_expression(0)?));
                        if self.peek().map(|t| &t.token) == Some(&Token::Comma) {
                            self.advance(); // consume comma
                        }
                        continue;
                    }
                    
                    // Parse comma-separated condition expressions
                    let mut conditions = Vec::new();
                    loop {
                        if let Some(cond) = self.parse_expression(0) {
                            conditions.push(cond);
                        }
                        if self.peek().map(|t| &t.token) == Some(&Token::Comma) {
                            self.advance();
                            // check if next is =>, then it's a trailing comma in conditions!
                            if self.peek().map(|t| &t.token) == Some(&Token::FatArrow) {
                                break;
                            }
                        } else {
                            break;
                        }
                    }
                    
                    self.match_token(Token::FatArrow);
                    let result = self.parse_expression(0)?;
                    arms.push((conditions, result));
                    
                    if self.peek().map(|t| &t.token) == Some(&Token::Comma) {
                        self.advance();
                    }
                }
                
                self.match_token(Token::CloseBrace);
                Some(Expr::Match {
                    subject: Box::new(subject),
                    arms,
                    default_arm,
                })
            }
            Token::Fn => {
                self.advance();
                self.match_token(Token::OpenParen);
                let params = self.parse_parameters();
                self.match_token(Token::CloseParen);
                self.skip_return_type_hint();
                if self.peek().map(|t| &t.token) == Some(&Token::FatArrow) {
                    self.advance();
                    let body = self.parse_expression(0)?;
                    Some(Expr::ArrowFunction { params, body: Box::new(body) })
                } else {
                    None
                }
            }
            Token::Function => {
                self.advance();
                self.match_token(Token::OpenParen);
                let params = self.parse_parameters();
                self.match_token(Token::CloseParen);
                
                let mut uses = Vec::new();
                if self.peek().map(|t| &t.token) == Some(&Token::Use) {
                    self.advance();
                    self.match_token(Token::OpenParen);
                    while let Some(p) = self.peek() {
                        if p.token == Token::CloseParen {
                            break;
                        }
                        if let Token::Ampersand = &p.token {
                            self.advance(); // consume '&'
                            if let Some(next_p) = self.peek() {
                                if let Token::Variable(name) = &next_p.token {
                                    uses.push((name.clone(), true));
                                    self.advance();
                                }
                            }
                        } else if let Token::Variable(name) = &p.token {
                            uses.push((name.clone(), false));
                            self.advance();
                        }
                        if self.peek().map(|t| &t.token) == Some(&Token::Comma) {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    self.match_token(Token::CloseParen);
                }
                self.skip_return_type_hint();
                self.match_token(Token::OpenBrace);
                self.function_depth += 1;
                let mut body = Vec::new();
                while let Some(peek) = self.peek() {
                    if peek.token == Token::CloseBrace || peek.token == Token::Eof {
                        break;
                    }
                    if let Some(stmt) = self.parse_statement() {
                        body.push(stmt);
                    }
                }
                self.function_depth -= 1;
                self.match_token(Token::CloseBrace);
                Some(Expr::Closure { params, uses, body })
            }
            Token::Not => {
                // Binds looser than instanceof (33) and `**`, tighter than `*` (30).
                let expr = self.parse_expression(32)?;
                Some(Expr::UnaryNot(Box::new(expr)))
            }
            Token::At => {
                let expr = self.parse_expression(35)?;
                Some(Expr::Silence(Box::new(expr)))
            }
            Token::Print => {
                let expr = self.parse_expression(0)?;
                Some(Expr::Print(Box::new(expr)))
            }
            Token::Isset => {
                self.match_token(Token::OpenParen);
                let mut vars = Vec::new();
                loop {
                    if let Some(expr) = self.parse_expression(0) {
                        vars.push(expr);
                    }
                    if self.peek().map(|t| &t.token) == Some(&Token::Comma) {
                        self.advance();
                    } else {
                        break;
                    }
                }
                self.match_token(Token::CloseParen);
                Some(Expr::Isset(vars))
            }
            Token::Empty => {
                self.match_token(Token::OpenParen);
                let expr = self.parse_expression(0)?;
                self.match_token(Token::CloseParen);
                Some(Expr::Empty(Box::new(expr)))
            }
            Token::Eval => {
                self.match_token(Token::OpenParen);
                let expr = self.parse_expression(0)?;
                self.match_token(Token::CloseParen);
                Some(Expr::Eval(Box::new(expr)))
            }
            Token::List => {
                self.match_token(Token::OpenParen);
                let mut vars = Vec::new();
                loop {
                    if self.peek().map(|t| &t.token) == Some(&Token::Comma) {
                        vars.push(None);
                        self.advance();
                        continue;
                    } else if self.peek().map(|t| &t.token) == Some(&Token::CloseParen) {
                        break;
                    }
                    if let Some(expr) = self.parse_expression(0) {
                        vars.push(Some(expr));
                    }
                    if self.peek().map(|t| &t.token) == Some(&Token::Comma) {
                        self.advance();
                    } else {
                        break;
                    }
                }
                self.match_token(Token::CloseParen);
                Some(Expr::List(vars))
            }
            Token::Minus => {
                let expr = self.parse_expression(35)?;
                Some(Expr::UnaryMinus(Box::new(expr)))
            }
            Token::PlusPlus => {
                let expr = self.parse_expression(35)?;
                Some(Expr::PreIncrement(Box::new(expr)))
            }
            Token::MinusMinus => {
                let expr = self.parse_expression(35)?;
                Some(Expr::PreDecrement(Box::new(expr)))
            }
            Token::OpenBracket => {
                let mut elements = Vec::new();
                while let Some(peek_record) = self.peek() {
                    if peek_record.token == Token::CloseBracket {
                        break;
                    }
                    if peek_record.token == Token::Comma {
                        self.advance();
                        elements.push((None, None));
                        continue;
                    }
                    if let Some(expr) = self.parse_expression(0) {
                        if self.match_token(Token::FatArrow) {
                            if let Some(val) = self.parse_expression(0) {
                                elements.push((Some(expr), Some(val)));
                            } else {
                                return None;
                            }
                        } else {
                            elements.push((None, Some(expr)));
                        }
                    }
                    if self.peek().map(|t| &t.token) == Some(&Token::Comma) {
                        self.advance();
                    }
                }
                if !self.match_token(Token::CloseBracket) {
                    return None;
                }
                Some(Expr::Array(elements))
            }
            Token::Clone => {
                let expr = self.parse_expression(0)?;
                Some(Expr::Clone(Box::new(expr)))
            }
            Token::Throw => {
                let expr = self.parse_expression(0)?;
                Some(Expr::Throw(Box::new(expr)))
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;

    #[test]
    fn test_parser_basic() {
        let source = "<?php $a = 42 + 3 * 5;";
        let lexer = Lexer::new(source);
        let mut parser = Parser::new(lexer);
        
        let program = parser.parse_program();
        assert_eq!(program.len(), 1);
        
        if let Stmt::ExprStmt(Expr::Assignment { target, value }) = &program[0] {
            assert_eq!(target, "a");
            if let Expr::BinaryOp { left, operator, right } = &**value {
                assert_eq!(**left, Expr::LiteralInt(42));
                assert_eq!(*operator, Token::Plus);
                
                if let Expr::BinaryOp { left: r_left, operator: r_op, right: r_right } = &**right {
                    assert_eq!(**r_left, Expr::LiteralInt(3));
                    assert_eq!(*r_op, Token::Multiply);
                    assert_eq!(**r_right, Expr::LiteralInt(5));
                } else {
                    panic!("Expected nested BinaryOp for multiplication");
                }
            } else {
                panic!("Expected BinaryOp for assignment value");
            }
        } else {
            panic!("Expected ExprStmt(Assignment) statement");
        }
    }

    #[test]
    fn test_inline_html_class() {
        let source = r#"<?php
class TestEmbed {
	public function maybe_run_ajax_cache() {
		?>
<script>
	jQuery( function($) {
		$.get("<?php echo 'foo'; ?>");
	} );
</script>
		<?php
	}

	public function register_handler() {
		return 123;
	}
}
"#;
        let lexer = Lexer::new(source);
        let mut parser = Parser::new(lexer);
        let program = parser.parse_program();
        assert_eq!(program.len(), 1);
        if let Stmt::Class { methods, .. } = &program[0] {
            assert_eq!(methods.len(), 2);
            if let Stmt::Function { name, .. } = &methods[0] {
                assert_eq!(name, "maybe_run_ajax_cache");
            }
            if let Stmt::Function { name, .. } = &methods[1] {
                assert_eq!(name, "register_handler");
            }
        } else {
            panic!("Expected Class");
        }
    }
}

