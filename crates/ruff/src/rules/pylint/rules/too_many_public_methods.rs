use ruff_python_ast::{self as ast, Stmt};

use ruff_diagnostics::{Diagnostic, Violation};
use ruff_macros::{derive_message_formats, violation};
use ruff_python_semantic::analyze::visibility::{self, Visibility::Public};

use crate::checkers::ast::Checker;

/// ## What it does
/// Checks for classes with too many public methods
///
/// By default, this rule allows up to 20 statements, as configured by the
/// `pylint.max-public-methods` option.
///
/// ## Why is this bad?
/// Classes with many public methods are harder to understand
/// and maintain.
///
/// Instead, consider refactoring the class into separate classes.
///
/// ## Example
/// With `pylint.max-public-settings` set to 5
///
/// ```python
/// class Linter:
///     def __init__(self):
///         pass
///
///     def pylint(self):
///         pass
///
///     def pylint_settings(self):
///         pass
///
///     def flake8(self):
///         pass
///
///     def flake8_settings(self):
///         pass
///
///     def pydocstyle(self):
///         pass
///
///     def pydocstyle_settings(self):
///         pass
/// ```
///
/// Use instead:
/// ```python
/// class Linter:
///     def __init__(self):
///         self.pylint = Pylint()
///         self.flake8 = Flake8()
///         self.pydocstyle = Pydocstyle()
///
///     def lint(self):
///         pass
///
///
/// class Pylint:
///     def lint(self):
///         pass
///
///     def settings(self):
///         pass
///
///
/// class Flake8:
///     def lint(self):
///         pass
///
///     def settings(self):
///         pass
///
///
/// class Pydocstyle:
///     def lint(self):
///         pass
///
///     def settings(self):
///         pass
/// ```
///
/// ## Options
/// - `pylint.max-public-methods`
#[violation]
pub struct TooManyPublicMethods {
    methods: usize,
    max_methods: usize,
}

impl Violation for TooManyPublicMethods {
    #[derive_message_formats]
    fn message(&self) -> String {
        let TooManyPublicMethods {
            methods,
            max_methods,
        } = self;
        format!("Too many public methods ({methods} > {max_methods})")
    }
}

/// R0904
pub(crate) fn too_many_public_methods(
    checker: &mut Checker,
    class_def: &ast::StmtClassDef,
    max_methods: usize,
) {
    let ast::StmtClassDef { body, range, .. } = class_def;
    let methods = body
        .iter()
        .filter(|stmt| match stmt {
            Stmt::FunctionDef(node) => matches!(visibility::method_visibility(node), Public),
            _ => false,
        })
        .count();

    if methods > max_methods {
        checker.diagnostics.push(Diagnostic::new(
            TooManyPublicMethods {
                methods,
                max_methods,
            },
            *range,
        ));
    }
}
