use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{PathBuf};
use datex_core::compiler::context::CompilationContext;
use datex_core::compiler::precompiler::AstWithMetadata;
use datex_core::compiler::scope::CompilationScope;
use crate::ast::parse;
use crate::compiler::error::CompilerError;
use crate::compiler::{compile_ast, precompile_to_ast_with_metadata};
use crate::runtime::Runtime;

/// Represents a file in the compiler workspace with its path, cached content and AST.
pub struct WorkspaceFile {
    pub path: PathBuf,
    pub content: String,
    pub ast_with_metadata: AstWithMetadata,
    pub compiled_dxb: Option<Vec<u8>>,
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

    /// Loads a file into the workspace, caching its content and AST.
    /// Returns a compiler error if parsing or precompilation fails.
    pub fn load_file(&mut self, path: PathBuf, content: String) -> Result<&WorkspaceFile, CompilerError> {
        let (ast_with_metadata, compilation_context) = self.get_ast_with_metadata_for_file(&path, content.clone())?;
        let workspace_file = WorkspaceFile {
            path: path.clone(),
            content,
            ast_with_metadata,
            compiled_dxb: Some(compilation_context.buffer.take())
        };
        self.files.insert(path.clone(), workspace_file);
        Ok(self.files.get(&path).unwrap())
    }

    /// Retrieves a reference to a workspace file by its path.
    pub fn get_file(&self, path: &PathBuf) -> Option<&WorkspaceFile> {
        self.files.get(path)
    }

    pub fn get_file_compiled_dxb(&self, path: &PathBuf) -> Option<&Vec<u8>> {
        self.get_file(path).and_then(|file| file.compiled_dxb.as_ref())
    }

    /// Retrieves the AST with metadata for a given file path and content after parsing and compilation.
    /// Returns a compiler error if parsing or compilation fails.
    fn get_ast_with_metadata_for_file(&self, path: &PathBuf, content: String) -> Result<(AstWithMetadata, CompilationContext), CompilerError> {
        let ast = parse(&content)?;
        let compilation_context = CompilationContext::new(
            RefCell::new(Vec::with_capacity(256)),
            &[],
            true
        );
        // FIXME: don't clone AST, optimize compile pipeline to avoid cloning
        let result = compile_ast(&compilation_context, ast.clone(), CompilationScope::default())?;
        Ok((
            AstWithMetadata {
                ast,
                metadata: result.precompiler_data.unwrap().ast_metadata,
            },
            compilation_context,
        ))
    }
}
