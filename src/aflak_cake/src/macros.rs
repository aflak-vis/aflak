use std::collections::{BTreeMap, HashMap};
use std::ops;
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::time::Instant;

use boow::Bow;

use super::{
    ConvertibleVariants, InputSlot, Output, Transform, TransformInputSlot, TypeId, VariantName, DST,
};
use compute::ComputeError;
use export::{DeserDST, ImportError, NamedAlgorithms};

pub struct MacroHandle<'t, T: 't, E: 't> {
    inner: Arc<RwLock<Macro<'t, T, E>>>,
}

impl<'t, T, E> Clone for MacroHandle<'t, T, E> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

pub struct Macro<'t, T: 't, E: 't> {
    id: usize,
    inputs: Vec<MacroInput<T>>,
    dst: DST<'t, T, E>,
    updated_on: Instant,
}

#[derive(Clone)]
struct MacroInput<T> {
    slot: InputSlot,
    type_id: Option<TypeId>,
    default: Option<T>,
}

impl<'t, T, E> Clone for Macro<'t, T, E>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            inputs: self.inputs.clone(),
            dst: self.dst.clone(),
            updated_on: self.updated_on,
        }
    }
}

impl<'t, T, E> MacroHandle<'t, T, E> {
    /// TODO
    pub fn name(&self) -> String {
        "Macro".to_owned()
    }

    pub fn input_types(&self) -> Vec<TypeId> {
        self.read().input_types()
    }

    pub fn outputs(&self) -> Vec<TypeId>
    where
        T: VariantName,
    {
        self.read().outputs()
    }

    pub fn inputs(&self) -> Vec<TransformInputSlot<T>>
    where
        T: Clone,
    {
        self.read().inputs()
    }

    pub fn defaults(&self) -> Vec<Option<T>>
    where
        T: Clone,
    {
        self.read().defaults()
    }

    pub fn id(&self) -> usize {
        self.read().id
    }

    pub fn call(&self, args: Vec<Bow<'_, T>>) -> Vec<Result<T, Arc<ComputeError<E>>>>
    where
        T: Clone + VariantName + ConvertibleVariants,
    {
        self.read().call(args)
    }

    pub fn get_macro(&self) -> Macro<'t, T, E>
    where
        T: Clone,
    {
        self.read().clone()
    }

    pub fn read(&self) -> RwLockReadGuard<'_, Macro<'t, T, E>> {
        self.inner.read().unwrap()
    }

    pub fn write(&self) -> MacroMut<'_, 't, T, E>
    where
        T: Clone,
    {
        MacroMut {
            inner: self.inner.write().unwrap(),
            changed: false,
        }
    }

    pub fn updated_on(&self) -> Instant {
        self.read().updated_on
    }
}

impl<'t, T, E> Macro<'t, T, E> {
    pub fn dst(&self) -> &DST<'t, T, E> {
        &self.dst
    }

    pub fn dst_mut(&mut self) -> &mut DST<'t, T, E> {
        &mut self.dst
    }

    fn input_types(&self) -> Vec<TypeId> {
        self.inputs
            .iter()
            .map(|input| input.type_id.unwrap_or(TypeId("No type")))
            .collect()
    }

    fn inputs(&self) -> Vec<TransformInputSlot<T>>
    where
        T: Clone,
    {
        self.inputs
            .iter()
            .map(|input| TransformInputSlot {
                // TODO
                type_id: input.type_id.unwrap_or(TypeId("No type")),
                default: input.default.clone(),
                name: "Macro input",
            })
            .collect()
    }

    fn outputs(&self) -> Vec<TypeId>
    where
        T: VariantName,
    {
        self.dst
            .outputs_iter()
            .map(|(_, some_output)| {
                if let Some(output) = some_output {
                    let t = self.dst.get_transform(output.t_idx).unwrap();
                    t.outputs()[output.index()]
                } else {
                    TypeId("No type")
                }
            })
            .collect()
    }

    fn defaults(&self) -> Vec<Option<T>>
    where
        T: Clone,
    {
        self.inputs
            .iter()
            .map(|input| &input.default)
            .cloned()
            .collect()
    }

    fn find_default_inputs(dst: &DST<'t, T, E>) -> Vec<MacroInput<T>>
    where
        T: Clone,
    {
        let mut inputs = vec![];
        for input_slot in dst.unattached_input_slots() {
            let input = match input_slot {
                InputSlot::Output(_) => MacroInput {
                    slot: input_slot,
                    type_id: None,
                    default: None,
                },
                InputSlot::Transform(input) => {
                    let t = dst.get_transform(input.t_idx).expect("Get transform");
                    let t_input = &t.inputs()[input.index()];
                    MacroInput {
                        slot: input_slot,
                        type_id: Some(t_input.type_id),
                        default: t_input.default.clone(),
                    }
                }
            };
            inputs.push(input);
        }
        inputs
    }

    fn call(&self, args: Vec<Bow<'_, T>>) -> Vec<Result<T, Arc<ComputeError<E>>>>
    where
        T: Clone + VariantName + ConvertibleVariants,
    {
        let mut cache = HashMap::new();

        let mut dst = self.dst.clone();

        for (arg, input) in args.into_iter().zip(&self.inputs) {
            let t_idx = dst.add_owned_transform(Transform::new_constant((*arg).clone()));
            let output = Output::new(t_idx, 0);
            match input.slot {
                InputSlot::Output(output_id) => dst.update_output(output_id, output),
                InputSlot::Transform(input) => {
                    if let Err(e) = dst.connect(output, input) {
                        unimplemented!("Type error: {}", e);
                    }
                }
            }
        }

        dst.outputs_iter()
            .map(|(output_id, _)| dst.compute_sync(*output_id, &mut cache))
            .collect()
    }
}

pub struct MacroMut<'a, 't: 'a, T: 't + Clone, E: 't> {
    inner: RwLockWriteGuard<'a, Macro<'t, T, E>>,
    changed: bool,
}

impl<'a, 't, T: Clone, E> Drop for MacroMut<'a, 't, T, E> {
    fn drop(&mut self) {
        if self.changed {
            self.inner.updated_on = Instant::now();
            self.inner.inputs = Macro::find_default_inputs(&self.inner.dst);
        }
    }
}

impl<'a, 't, T: Clone, E> ops::Deref for MacroMut<'a, 't, T, E> {
    type Target = Macro<'t, T, E>;
    fn deref(&self) -> &Self::Target {
        self.inner.deref()
    }
}

impl<'a, 't, T: Clone, E> ops::DerefMut for MacroMut<'a, 't, T, E> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.changed = true;
        self.inner.deref_mut()
    }
}

pub struct MacroManager<'t, T: 't, E: 't> {
    cnt: usize,
    macros: BTreeMap<usize, MacroHandle<'t, T, E>>,
}

impl<'t, T, E> MacroManager<'t, T, E> {
    pub fn get_macro(&self, id: usize) -> Option<&MacroHandle<'t, T, E>> {
        self.macros.get(&id)
    }

    pub fn new() -> Self {
        MacroManager {
            cnt: 0,
            macros: BTreeMap::new(),
        }
    }

    pub fn create_macro(&mut self) -> &MacroHandle<'t, T, E>
    where
        T: Clone,
    {
        self.cnt += 1;
        let dst = DST::new();
        self.macros.insert(
            self.cnt,
            MacroHandle {
                inner: Arc::new(RwLock::new(Macro {
                    id: self.cnt,
                    inputs: Macro::find_default_inputs(&dst),
                    dst,
                    updated_on: Instant::now(),
                })),
            },
        );
        self.macros.get(&self.cnt).unwrap()
    }

    pub fn to_serializable(&self) -> SerdeMacroManager<T>
    where
        T: Clone + VariantName,
    {
        SerdeMacroManager::from(self)
    }

    pub fn macros(&self) -> impl Iterator<Item = &MacroHandle<'t, T, E>> {
        self.macros.values()
    }
}

impl<T, E> MacroManager<'static, T, E> {
    pub fn from_deserializable(&mut self, deser: SerdeMacroManager<T>) -> Result<(), ImportError>
    where
        T: Clone + VariantName + ConvertibleVariants + NamedAlgorithms<E>,
    {
        deser
            .into_macro_manager(&MacroManager::new())
            .map(|new_manager| {
                *self = new_manager;
            })
    }
}

impl<'a, 't, T, E> From<&'a MacroManager<'t, T, E>> for SerdeMacroManager<T>
where
    T: Clone + VariantName,
{
    fn from(manager: &'a MacroManager<'t, T, E>) -> Self {
        Self {
            macros: manager
                .macros
                .values()
                .map(|handle| SerdeMacro::from(&*handle.read()))
                .collect(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct SerdeMacro<T> {
    id: usize,
    inputs: Vec<SerdeMacroInput<T>>,
    dst: DeserDST<T>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct SerdeMacroInput<T> {
    slot: InputSlot,
    type_id: Option<String>,
    default: Option<T>,
}

impl<'t, 'd, T, E> From<&'d Macro<'t, T, E>> for SerdeMacro<T>
where
    T: Clone,
{
    fn from(macr: &'d Macro<'t, T, E>) -> Self {
        Self {
            id: macr.id,
            inputs: macr.inputs.iter().map(SerdeMacroInput::from).collect(),
            dst: DeserDST::from_dst(&macr.dst),
        }
    }
}

impl<T> SerdeMacro<T> {
    fn into_macro<E>(
        self,
        macro_manager: &MacroManager<'static, T, E>,
    ) -> Result<Macro<'static, T, E>, ImportError>
    where
        T: Clone + VariantName + ConvertibleVariants + NamedAlgorithms<E>,
    {
        // TODO: Deal with nested macros
        let id = self.id;
        let mut inputs = Vec::with_capacity(self.inputs.len());
        for input in self.inputs {
            inputs.push(input.into_macro_input()?);
        }
        self.dst.into_dst(macro_manager).map(move |dst| Macro {
            id,
            inputs,
            dst,
            updated_on: Instant::now(),
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SerdeMacroManager<T> {
    macros: Vec<SerdeMacro<T>>,
}

impl<T> SerdeMacroManager<T> {
    fn into_macro_manager<E>(
        self,
        macro_manager: &MacroManager<'static, T, E>,
    ) -> Result<MacroManager<'static, T, E>, ImportError>
    where
        T: Clone + VariantName + ConvertibleVariants + NamedAlgorithms<E>,
    {
        let cnt = self.macros.iter().map(|macr| macr.id).max().unwrap_or(0);
        let mut macros = BTreeMap::new();
        for macr in self.macros {
            let macr = macr.into_macro(macro_manager)?;
            macros.insert(
                macr.id,
                MacroHandle {
                    inner: Arc::new(RwLock::new(macr)),
                },
            );
        }
        Ok(MacroManager { cnt, macros })
    }
}

impl<T> SerdeMacroInput<T>
where
    T: VariantName,
{
    fn into_macro_input(self) -> Result<MacroInput<T>, ImportError> {
        let type_id = if let Some(variant_name) = self.type_id {
            let some_type_id = T::variant_names()
                .iter()
                .find(|name| **name == variant_name);
            if let Some(type_id) = some_type_id {
                Some(TypeId(type_id))
            } else {
                return Err(ImportError::UnexpectedType(variant_name));
            }
        } else {
            None
        };
        Ok(MacroInput {
            slot: self.slot,
            type_id,
            default: self.default,
        })
    }
}

impl<'a, T> From<&'a MacroInput<T>> for SerdeMacroInput<T>
where
    T: Clone,
{
    fn from(input: &'a MacroInput<T>) -> Self {
        Self {
            slot: input.slot,
            type_id: input.type_id.map(|type_id| type_id.name().to_owned()),
            default: input.default.clone(),
        }
    }
}
