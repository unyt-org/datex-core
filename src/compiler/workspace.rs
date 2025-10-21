use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{PathBuf};
use std::rc::Rc;
use datex_core::compiler::precompiler::{RichAst};
use crate::compiler::error::{CompilerError, DetailedCompilerErrors, SpannedCompilerError};
use crate::compiler::{parse_datex_script_to_rich_ast_detailed_errors, CompileOptions};
use crate::compiler::type_inference::infer_expression_type_inner;
use crate::runtime::Runtime;
use crate::compiler::error::{DetailedCompilerErrorsWithMaybeRichAst, DetailedCompilerErrorsWithRichAst};
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
#[derive(Default)]
pub struct CompilerWorkspace {
    files: HashMap<PathBuf, WorkspaceFile>,
    runtime: Runtime
}


impl CompilerWorkspace {
    /// Creates a new compiler workspace with the given runtime.
    pub fn new(runtime: Runtime) -> Self {
        Self {
            files: HashMap::new(),
            runtime
        }
    }
    
    pub fn files(&self) -> &HashMap<PathBuf, WorkspaceFile> {
        &self.files
    }

    /// Loads a file into the workspace, caching its content and AST.
    /// Returns a compiler error if parsing or precompilation fails.
    pub fn load_file(&mut self, path: PathBuf, content: String) -> &WorkspaceFile {
        let result = self.get_rich_ast_for_file(&path, content.clone());
        let workspace_file = match result {
            Ok((rich_ast, return_type)) => WorkspaceFile {
                path: path.clone(),
                content,
                rich_ast: Some(rich_ast),
                return_type: Some(return_type),
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

    /// Retrieves the AST with metadata for a given file path and content after parsing and compilation.
    /// Returns a compiler error if parsing or compilation fails.
    fn get_rich_ast_for_file(&self, path: &PathBuf, content: String) -> Result<(RichAst, TypeContainer), DetailedCompilerErrorsWithMaybeRichAst> {
        let mut options = CompileOptions::default();
        let mut rich_ast = parse_datex_script_to_rich_ast_detailed_errors(&content, &mut options)?;
        let return_type = infer_expression_type_inner(rich_ast.ast.as_mut().unwrap(), rich_ast.metadata.clone())
            // TODO: detailed type errors
            .map_err(|e| DetailedCompilerErrorsWithRichAst {
                errors: DetailedCompilerErrors {errors: vec![SpannedCompilerError::from(CompilerError::TypeError(e))]},
                // TODO: only temporary fake ast
                ast: rich_ast.clone()
            })?;
        Ok((
            rich_ast,
            return_type
        ))
    }
}
