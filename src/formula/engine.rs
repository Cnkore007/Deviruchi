use rhai::{AST, Engine, Scope};
use std::path::Path;

pub struct FormulaEngine {
    engine: Engine,
    battle_ast: Option<AST>,
    status_ast: Option<AST>,
}

impl FormulaEngine {
    pub fn new() -> Self {
        let mut engine = Engine::new();
        engine.register_fn("min", |a: i64, b: i64| a.min(b));
        engine.register_fn("max", |a: i64, b: i64| a.max(b));
        engine.register_fn("sqrt", |a: f64| a.sqrt());
        engine.register_fn("clamp", |val: i64, lo: i64, hi: i64| val.clamp(lo, hi));

        let mut this = Self {
            engine,
            battle_ast: None,
            status_ast: None,
        };
        this.load_scripts();
        this
    }

    fn load_scripts(&mut self) {
        let battle_path = "db/formulas/battle.rhai";
        if Path::new(battle_path).exists() {
            match self.engine.compile_file(battle_path.into()) {
                Ok(ast) => self.battle_ast = Some(ast),
                Err(e) => tracing::warn!("加载 battle.rhai 失败: {}", e),
            }
        }
        let status_path = "db/formulas/status.rhai";
        if Path::new(status_path).exists() {
            match self.engine.compile_file(status_path.into()) {
                Ok(ast) => self.status_ast = Some(ast),
                Err(e) => tracing::warn!("加载 status.rhai 失败: {}", e),
            }
        }
    }

    pub fn has_battle_formulas(&self) -> bool {
        self.battle_ast.is_some()
    }
    pub fn has_status_formulas(&self) -> bool {
        self.status_ast.is_some()
    }

    pub fn call_battle_fn(&self, name: &str, args: Vec<rhai::Dynamic>) -> Option<rhai::Dynamic> {
        let ast = self.battle_ast.as_ref()?;
        self.engine
            .call_fn::<rhai::Dynamic>(&mut Scope::new(), ast, name, args)
            .ok()
    }

    pub fn call_status_fn(&self, name: &str, args: Vec<rhai::Dynamic>) -> Option<rhai::Dynamic> {
        let ast = self.status_ast.as_ref()?;
        self.engine
            .call_fn::<rhai::Dynamic>(&mut Scope::new(), ast, name, args)
            .ok()
    }
}

impl Default for FormulaEngine {
    fn default() -> Self {
        Self::new()
    }
}
