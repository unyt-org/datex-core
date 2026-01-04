use url::Url;

use crate::collections::HashMap;
use crate::compiler::error::DetailedCompilerErrors;
use crate::compiler::error::DetailedCompilerErrorsWithMaybeRichAst;
use crate::compiler::precompiler::precompiled_ast::RichAst;
use crate::compiler::{
    CompileOptions, parse_datex_script_to_rich_ast_detailed_errors,
};
use crate::runtime::Runtime;
use crate::values::core_values::r#type::Type;

/// Represents a file in the compiler workspace with its URL, cached content and AST.
pub struct WorkspaceFile {
    pub url: Url,
    pub content: String,
    pub rich_ast: Option<RichAst>,
    pub return_type: Option<Type>,
    pub errors: Option<DetailedCompilerErrors>,
}

/// Represents the compiler workspace containing multiple files.
pub struct CompilerWorkspace {
    files: HashMap<Url, WorkspaceFile>,
    runtime: Runtime,
}

impl CompilerWorkspace {
    /// Creates a new compiler workspace with the given runtime.
    pub fn new(runtime: Runtime) -> Self {
        Self {
            files: HashMap::new(),
            runtime,
        }
    }

    pub fn files(&self) -> &HashMap<Url, WorkspaceFile> {
        &self.files
    }

    /// Loads a file into the workspace, caching its content and AST.
    /// Returns a compiler error if parsing or precompilation fails.
    pub fn load_file(&mut self, url: Url, content: String) -> &WorkspaceFile {
        let result = self.get_rich_ast_for_file(&url, content.clone());
        let workspace_file = match result {
            Ok(rich_ast) => WorkspaceFile {
                url: url.clone(),
                content,
                rich_ast: Some(rich_ast),
                return_type: None,
                errors: None,
            },
            Err(error) => WorkspaceFile {
                url: url.clone(),
                content,
                rich_ast: error.ast,
                return_type: None,
                errors: Some(error.errors),
            },
        };
        self.files.insert(url.clone(), workspace_file);
        self.files.get(&url).unwrap()
    }

    /// Retrieves a reference to a workspace file by its URL.
    pub fn get_file(&self, url: &Url) -> Option<&WorkspaceFile> {
        self.files.get(url)
    }

    pub fn get_file_mut(&mut self, url: &Url) -> Option<&mut WorkspaceFile> {
        self.files.get_mut(url)
    }

    /// Retrieves the AST with metadata for a given file path and content after parsing and compilation.
    /// Returns a compiler error if parsing or compilation fails.
    fn get_rich_ast_for_file(
        &self,
        url: &Url,
        content: String,
    ) -> Result<RichAst, DetailedCompilerErrorsWithMaybeRichAst> {
        let mut options = CompileOptions::default();
        let rich_ast = parse_datex_script_to_rich_ast_detailed_errors(
            &content,
            &mut options,
        )?;
        Ok(rich_ast)
    }
}
