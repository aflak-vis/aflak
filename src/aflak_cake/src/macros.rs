use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};

use boow::Bow;

use super::{TransformInputSlot, TypeId};
use compute::ComputeError;

#[derive(Clone)]
pub struct MacroHandle {
    inner: Arc<RwLock<Macro>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Macro {
    id: usize,
}

/// TODO
impl MacroHandle {
    pub fn name(&self) -> String {
        "Macro".to_owned()
    }

    pub fn input_types(&self) -> Vec<TypeId> {
        vec![]
    }

    pub fn outputs(&self) -> Vec<TypeId> {
        vec![]
    }

    pub fn inputs<T>(&self) -> Vec<TransformInputSlot<T>> {
        vec![]
    }

    pub fn defaults<T>(&self) -> Vec<Option<T>> {
        vec![]
    }

    pub fn id(&self) -> usize {
        self.inner.read().unwrap().id
    }

    pub fn call<T, E>(&self, _args: Vec<Bow<'_, T>>) -> Vec<Result<T, Arc<ComputeError<E>>>> {
        vec![]
    }

    pub fn get_macro(&self) -> Macro {
        self.inner.read().unwrap().clone()
    }
}

pub struct MacroManager {
    cnt: usize,
    macros: BTreeMap<usize, MacroHandle>,
}

impl MacroManager {
    pub fn get_macro(&self, id: usize) -> Option<&MacroHandle> {
        self.macros.get(&id)
    }

    pub fn new() -> MacroManager {
        MacroManager {
            cnt: 0,
            macros: BTreeMap::new(),
        }
    }

    pub fn create_macro(&mut self) -> &MacroHandle {
        self.cnt += 1;
        self.macros.insert(
            self.cnt,
            MacroHandle {
                inner: Arc::new(RwLock::new(Macro { id: self.cnt })),
            },
        );
        self.macros.get(&self.cnt).unwrap()
    }

    pub fn to_serializable(&self) -> SerdeMacroManager {
        SerdeMacroManager::from(self)
    }

    pub fn from_deserializable(&mut self, deser: SerdeMacroManager) {
        *self = Self::from(deser);
    }
}

impl From<SerdeMacroManager> for MacroManager {
    fn from(deser: SerdeMacroManager) -> MacroManager {
        let cnt = deser.macros.iter().map(|macr| macr.id).max().unwrap_or(0);
        let mut macros = BTreeMap::new();
        for macr in deser.macros {
            macros.insert(
                macr.id,
                MacroHandle {
                    inner: Arc::new(RwLock::new(macr)),
                },
            );
        }
        MacroManager { cnt, macros }
    }
}

impl<'a> From<&'a MacroManager> for SerdeMacroManager {
    fn from(manager: &'a MacroManager) -> SerdeMacroManager {
        SerdeMacroManager {
            macros: manager
                .macros
                .values()
                .map(|handle| handle.get_macro())
                .collect(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SerdeMacroManager {
    macros: Vec<Macro>,
}
