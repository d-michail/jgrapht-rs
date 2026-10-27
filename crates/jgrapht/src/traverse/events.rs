/// Receives traversal events (JGraphT's `TraversalListener`). All methods default to no-ops.
#[allow(unused_variables)]
pub trait TraversalListener<V, E> {
    fn connected_component_started(&mut self) {}
    fn connected_component_finished(&mut self) {}
    fn edge_traversed(&mut self, e: &E) {}
    fn vertex_traversed(&mut self, v: &V) {}
    fn vertex_finished(&mut self, v: &V) {}
}

/// A set of listeners with ids, used by the iterators.
pub struct TraversalListeners<'g, V, E> {
    list: Vec<(usize, Box<dyn TraversalListener<V, E> + 'g>)>,
    next_id: usize,
}

impl<V, E> Default for TraversalListeners<'_, V, E> {
    fn default() -> Self {
        TraversalListeners {
            list: Vec::new(),
            next_id: 0,
        }
    }
}

impl<'g, V, E> TraversalListeners<'g, V, E> {
    pub fn add(&mut self, l: impl TraversalListener<V, E> + 'g) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        self.list.push((id, Box::new(l)));
        id
    }
    pub fn remove(&mut self, id: usize) -> bool {
        let before = self.list.len();
        self.list.retain(|(i, _)| *i != id);
        before != self.list.len()
    }
    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }
    pub(crate) fn component_started(&mut self) {
        self.list
            .iter_mut()
            .for_each(|(_, l)| l.connected_component_started());
    }
    pub(crate) fn component_finished(&mut self) {
        self.list
            .iter_mut()
            .for_each(|(_, l)| l.connected_component_finished());
    }
    pub(crate) fn edge_traversed(&mut self, e: &E) {
        self.list.iter_mut().for_each(|(_, l)| l.edge_traversed(e));
    }
    pub(crate) fn vertex_traversed(&mut self, v: &V) {
        self.list
            .iter_mut()
            .for_each(|(_, l)| l.vertex_traversed(v));
    }
    pub(crate) fn vertex_finished(&mut self, v: &V) {
        self.list.iter_mut().for_each(|(_, l)| l.vertex_finished(v));
    }
}
