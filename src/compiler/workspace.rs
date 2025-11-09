use crate::compiler::error::DetailedCompilerErrors;
use crate::compiler::error::DetailedCompilerErrorsWithMaybeRichAst;
use crate::compiler::precompiler::precompiled_ast::RichAst;
use crate::compiler::{
    CompileOptions, parse_datex_script_to_rich_ast_detailed_errors,
};
use crate::runtime::Runtime;
use crate::collections::HashMap;
use crate::stdlib::path::PathBuf;
use crate::types::type_container::TypeContainer;

/// Represents a file in the compiler workspace with its path, cached content and AST.
pub struct WorkspaceFile {
    pub path: PathBuf,
    pub content: String,
    pub rich_ast: Option<RichAst>,
    pub return_type: Option<TypeContainer>,
    pub errors: Option<DetailedCompilerErrors>,
}

/// Represents the compiler workspace containing multiple files.
pub struct CompilerWorkspace {
    files: HashMap<PathBuf, WorkspaceFile>,
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

    pub fn files(&self) -> &HashMap<PathBuf, WorkspaceFile> {
        &self.files
    }

    /// Loads a file into the workspace, caching its content and AST.
    /// Returns a compiler error if parsing or precompilation fails.
    pub fn load_file(
        &mut self,
        path: PathBuf,
        content: String,
    ) -> &WorkspaceFile {
        let result = self.get_rich_ast_for_file(&path, content.clone());
        let workspace_file = match result {
            Ok(rich_ast) => WorkspaceFile {
                path: path.clone(),
                content,
                rich_ast: Some(rich_ast),
                return_type: None,
                errors: None,
            },
            Err(error) => WorkspaceFile {
                path: path.clone(),
                content,
                rich_ast: error.ast,
                return_type: None,
                errors: Some(error.errors),
            },
        };
        self.files.insert(path.clone(), workspace_file);
        self.files.get(&path).unwrap()
    }

    /// Retrieves a reference to a workspace file by its path.
    pub fn get_file(&self, path: &PathBuf) -> Option<&WorkspaceFile> {
        self.files.get(path)
    }

    pub fn get_file_mut(
        &mut self,
        path: &PathBuf,
    ) -> Option<&mut WorkspaceFile> {
        self.files.get_mut(path)
    }

    /// Retrieves the AST with metadata for a given file path and content after parsing and compilation.
    /// Returns a compiler error if parsing or compilation fails.
    fn get_rich_ast_for_file(
        &self,
        path: &PathBuf,
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
