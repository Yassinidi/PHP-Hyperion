use crate::lexer::Token;

#[derive(Debug, PartialEq, Clone)]
pub struct ParamDef {
    pub name: String,
    pub type_hint: Option<String>,
    pub has_default: bool,
    pub default_expr: Option<Expr>,
    pub is_variadic: bool,
    /// `function f(&$x)` — the caller's slot is aliased, not copied.
    pub by_ref: bool,
    pub promoted_visibility: Option<Visibility>,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Visibility {
    Public,
    Protected,
    Private,
}

#[derive(Debug, PartialEq, Clone)]
pub struct CatchBlock {
    pub types: Vec<String>,
    pub var: Option<String>,
    pub body: Vec<Stmt>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum Expr {
    LiteralInt(i64),
    LiteralFloat(f64),
    LiteralString(String),
    InterpolatedString(Vec<Expr>),
    LiteralNull,
    LiteralBool(bool),
    Identifier(String),
    Variable(String),
    VariableVariable(Box<Expr>),
    /// `&$x` — yields a reference to the operand's slot rather than its value.
    /// Valid as an assignment RHS (`$a = &$b`) and as a call argument.
    MakeRef(Box<Expr>),
    FirstClassCallable(Box<Expr>),
    Assignment {
        target: String,
        value: Box<Expr>,
    },
    VariableVariableAssign {
        target: Box<Expr>,
        value: Box<Expr>,
    },
    BinaryOp {
        left: Box<Expr>,
        operator: Token,
        right: Box<Expr>,
    },
    Call {
        callee: Box<Expr>,
        arguments: Vec<Expr>,
    },
    MethodCall {
        object: Box<Expr>,
        method: String,
        arguments: Vec<Expr>,
    },
    StaticMethodCall {
        class_name: String,
        method: String,
        arguments: Vec<Expr>,
    },
    StaticPropertyGet {
        class_name: String,
        property: String,
    },
    PropertyGet {
        object: Box<Expr>,
        property: Box<Expr>,
    },
    NullCoalesce {
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Ternary {
        condition: Box<Expr>,
        true_expr: Box<Expr>,
        false_expr: Box<Expr>,
    },
    Elvis {
        condition: Box<Expr>,
        false_expr: Box<Expr>,
    },
    NullsafePropertyGet {
        object: Box<Expr>,
        property: Box<Expr>,
    },
    NullsafeMethodCall {
        object: Box<Expr>,
        method: String,
        arguments: Vec<Expr>,
    },
    Array(Vec<(Option<Expr>, Option<Expr>)>),
    PropertySet {
        object: Box<Expr>,
        property: Box<Expr>,
        value: Box<Expr>,
    },
    StaticPropertySet {
        class_name: String,
        property: String,
        value: Box<Expr>,
    },
    ArraySet {
        array: Box<Expr>,
        key: Option<Box<Expr>>,
        value: Box<Expr>,
    },
    ArrayDestructure {
        elements: Vec<(Option<Expr>, Option<Expr>)>,
        value: Box<Expr>,
    },
    ListDestructure {
        vars: Vec<Option<Expr>>,
        value: Box<Expr>,
    },
    Closure {
        params: Vec<ParamDef>,
        uses: Vec<(String, bool)>,
        body: Vec<Stmt>,
    },
    ArrowFunction {
        params: Vec<ParamDef>,
        body: Box<Expr>,
    },
    Match {
        subject: Box<Expr>,
        arms: Vec<(Vec<Expr>, Expr)>,
        default_arm: Option<Box<Expr>>,
    },
    NamedArgument {
        name: String,
        value: Box<Expr>,
    },
    LateStaticMethodCall {
        method: String,
        arguments: Vec<Expr>,
    },
    DynamicStaticMethodCall { class_name: Box<Expr>, method: Box<Expr>, arguments: Vec<Expr> },
    DynamicMethodCall {
        object: Box<Expr>,
        method: Box<Expr>,
        arguments: Vec<Expr>,
    },
    DynamicNullsafeMethodCall {
        object: Box<Expr>,
        method: Box<Expr>,
        arguments: Vec<Expr>,
    },
    LateStaticPropertyGet {
        property: String,
    },
    New {
        class_name: String,
        arguments: Vec<Expr>,
    },
    NewDynamic {
        class_expr: Box<Expr>,
        arguments: Vec<Expr>,
    },
    ArrayGet {
        array: Box<Expr>,
        key: Option<Box<Expr>>,
    },
    Require(Box<Expr>),
    Include(Box<Expr>),
    RequireOnce(Box<Expr>),
    IncludeOnce(Box<Expr>),
    UnaryNot(Box<Expr>),
    UnaryMinus(Box<Expr>),
    CastInt(Box<Expr>),
    CastString(Box<Expr>),
    CastBool(Box<Expr>),
    CastFloat(Box<Expr>),
    CastArray(Box<Expr>),
    CastObject(Box<Expr>),
    NewAnonymousClass {
        extends: Option<String>,
        implements: Vec<String>,
        uses: Vec<String>,
        methods: Vec<Stmt>,
        properties: Vec<Stmt>,
        arguments: Vec<Expr>,
    },
    ClassConstFetch {
        class_name: String,
        constant_name: String,
    },
    Yield {
        key: Option<Box<Expr>>,
        value: Option<Box<Expr>>,
    },
    YieldFrom(Box<Expr>),
    Clone(Box<Expr>),
    Unpack(Box<Expr>),
    Isset(Vec<Expr>),
    Empty(Box<Expr>),
    List(Vec<Option<Expr>>),
    Throw(Box<Expr>),
    InstanceOf {
        object: Box<Expr>,
        class_name: Box<Expr>,
    },
    BitwiseAnd(Box<Expr>, Box<Expr>),
    BitwiseOr(Box<Expr>, Box<Expr>),
    BitwiseXor(Box<Expr>, Box<Expr>),
    BitwiseNot(Box<Expr>),
    ShiftLeft(Box<Expr>, Box<Expr>),
    ShiftRight(Box<Expr>, Box<Expr>),
    PreIncrement(Box<Expr>),
    PostIncrement(Box<Expr>),
    PreDecrement(Box<Expr>),
    PostDecrement(Box<Expr>),
    CompoundAssign {
        target: Box<Expr>,
        operator: Token,
        value: Box<Expr>,
    },
    Print(Box<Expr>),
    Silence(Box<Expr>),
    Eval(Box<Expr>),
}

#[derive(Debug, PartialEq, Clone)]
pub struct Attribute {
    pub name: String,
    pub arguments: Vec<Expr>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum Stmt {
    ExprStmt(Expr),
    If {
        condition: Expr,
        then_branch: Vec<Stmt>,
        else_branch: Option<Vec<Stmt>>,
    },
    While {
        condition: Expr,
        body: Vec<Stmt>,
    },
    Foreach {
        iterable: Expr,
        key_var: Option<String>,
        value_var: String,
        /// `foreach ($a as &$v)` — writes to `$v` mutate the source array.
        value_by_ref: bool,
        body: Vec<Stmt>,
    },
    For {
        init: Vec<Expr>,
        condition: Vec<Expr>,
        increment: Vec<Expr>,
        body: Vec<Stmt>,
    },
    Function {
        name: String,
        params: Vec<ParamDef>,
        body: Vec<Stmt>,
        is_static: bool,
        visibility: Visibility,
        attributes: Vec<Attribute>,
    },
    Return(Expr),
    Echo(Vec<Expr>),
    Class {
        name: String,
        is_abstract: bool,
        is_final: bool,
        is_readonly: bool,
        extends: Option<String>,
        methods: Vec<Stmt>,
        properties: Vec<Stmt>,
        implements: Vec<String>,
        uses: Vec<String>,
        trait_aliases: Vec<(Option<String>, String, String)>,
        attributes: Vec<Attribute>,
    },
    Enum {
        name: String,
        backed_type: Option<String>,
        implements: Vec<String>,
        cases: Vec<(String, Option<Expr>)>,
        methods: Vec<Stmt>,
    },
    Namespace(String),
    Use {
        class_name: String,
        alias: Option<String>,
    },
    Trait {
        name: String,
        uses: Vec<String>,
        trait_aliases: Vec<(Option<String>, String, String)>,
        methods: Vec<Stmt>,
        properties: Vec<Stmt>,
    },
    Interface {
        name: String,
        extends: Vec<String>,
        methods: Vec<Stmt>,
    },
    Switch {
        condition: Expr,
        cases: Vec<(Expr, Vec<Stmt>)>,
        default: Option<Vec<Stmt>>,
    },
    TryCatch {
        try_body: Vec<Stmt>,
        catches: Vec<CatchBlock>,
        finally_body: Option<Vec<Stmt>>,
    },
    Throw(Expr),
    Break(Option<u32>),
    PropertyDeclaration {
        name: String,
        initial_value: Option<Expr>,
        is_static: bool,
        is_readonly: bool,
        visibility: Visibility,
    },
    /// `const NAME = expr;` — at file/namespace scope, or in a class, interface,
    /// trait or enum body.
    ///
    /// Kept separate from `PropertyDeclaration` because a constant is not a
    /// property: it has no `$`, it is reached as `Foo::NAME` rather than through
    /// an instance, and it cannot be assigned to. Folding the two together is
    /// what made `self::COLORS` read back null in Symfony's `Color` class — the
    /// const landed in the instance-property table that `Foo::NAME` never
    /// consults.
    ConstDeclaration {
        name: String,
        value: Expr,
        visibility: Visibility,
    },
    Block(Vec<Stmt>),
    DoWhile {
        body: Vec<Stmt>,
        condition: Expr,
    },
    Continue(Option<u32>),
    GlobalVar(Vec<String>),
    StaticVar(Vec<(String, Option<Expr>)>),
    Unset(Vec<Expr>),
}

// A full PHP program is just a list of statements
pub type Program = Vec<Stmt>;
