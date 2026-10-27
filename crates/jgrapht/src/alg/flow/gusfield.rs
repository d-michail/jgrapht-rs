use super::base::{MaximumFlow, MaximumFlowAlgorithm, MinimumSTCutAlgorithm};
use super::{DEFAULT_EPSILON, PushRelabelMFImpl};
use crate::error::{GraphError, Result};
use crate::graph::props::require_undirected;
use crate::graph::{DefaultEdge, Graph, GraphExt, GraphMut, GraphMutExt, UGraph};
use indexmap::IndexSet;
use std::collections::{HashMap, VecDeque};

type CutAlgorithm<'g, G> =
    Box<dyn MinimumSTCutAlgorithm<<G as Graph>::Vertex, <G as Graph>::Edge> + 'g>;

fn unsupported<T>() -> Result<T> {
    Err(GraphError::Unsupported(
        "Flows calculated via Equivalent Flow trees only provide a maximum flow value, not the exact flow per edge/arc."
            .into(),
    ))
}

/// Shared set-up of the two Gusfield algorithms: the vertex order and index map.
fn index_vertices<G: Graph>(network: &G) -> Result<(Vec<G::Vertex>, HashMap<G::Vertex, usize>)> {
    if network.vertex_count() < 2 {
        return Err(GraphError::InvalidArgument(
            "Graph must have at least 2 vertices".into(),
        ));
    }
    let vertex_list: Vec<G::Vertex> = network.vertices().cloned().collect();
    let index_map = vertex_list
        .iter()
        .enumerate()
        .map(|(i, v)| (v.clone(), i))
        .collect();
    Ok((vertex_list, index_map))
}

/// `GusfieldEquivalentFlowTree`: all-pairs maximum flow values from `n - 1` minimum cut
/// computations, kept in an equivalent flow tree.
pub struct GusfieldEquivalentFlowTree<'g, G: Graph> {
    n: usize,
    minimum_st_cut_algorithm: CutAlgorithm<'g, G>,
    vertex_list: Vec<G::Vertex>,
    index_map: HashMap<G::Vertex, usize>,
    p: Option<Vec<usize>>,
    neighbors: Vec<usize>,
    flow_matrix: Vec<Vec<f64>>,
}

impl<'g, G: Graph + 'g> GusfieldEquivalentFlowTree<'g, G> {
    pub fn new(network: &'g G) -> Result<Self> {
        Self::with_epsilon(network, DEFAULT_EPSILON)
    }

    pub fn with_epsilon(network: &'g G, epsilon: f64) -> Result<Self> {
        Self::with_algorithm(
            network,
            Box::new(PushRelabelMFImpl::with_epsilon(network, epsilon)?),
        )
    }

    /// Uses `minimum_st_cut_algorithm` for the cut computations.
    pub fn with_algorithm(
        network: &'g G,
        minimum_st_cut_algorithm: CutAlgorithm<'g, G>,
    ) -> Result<Self> {
        require_undirected(network)?;
        let (vertex_list, index_map) = index_vertices(network)?;
        Ok(GusfieldEquivalentFlowTree {
            n: vertex_list.len(),
            minimum_st_cut_algorithm,
            vertex_list,
            index_map,
            p: None,
            neighbors: Vec::new(),
            flow_matrix: Vec::new(),
        })
    }

    fn calculate_equivalent_flow_tree(&mut self) -> Result<()> {
        let n = self.n;
        let mut flow_matrix = vec![vec![0.0; n]; n];
        let mut p = vec![0usize; n];
        let mut neighbors = vec![0usize; n];
        for s in 1..n {
            let t = p[s];
            neighbors[s] = t;
            let flow_value = self
                .minimum_st_cut_algorithm
                .calculate_min_cut(&self.vertex_list[s], &self.vertex_list[t])?;
            let source_partition = self.minimum_st_cut_algorithm.source_partition();
            for i in s..n {
                if source_partition.contains(&self.vertex_list[i]) && p[i] == t {
                    p[i] = s;
                }
            }
            flow_matrix[s][t] = flow_value;
            flow_matrix[t][s] = flow_value;
            for i in 0..s {
                if i != t {
                    let v = flow_matrix[s][t].min(flow_matrix[t][i]);
                    flow_matrix[s][i] = v;
                    flow_matrix[i][s] = v;
                }
            }
        }
        self.flow_matrix = flow_matrix;
        self.p = Some(p);
        self.neighbors = neighbors;
        Ok(())
    }

    /// The equivalent flow tree: an edge `(i, neighbor(i))` weighted with their maximum flow value.
    pub fn equivalent_flow_tree(&mut self) -> Result<UGraph<G::Vertex>> {
        if self.p.is_none() {
            self.calculate_equivalent_flow_tree()?;
        }
        let mut tree = UGraph::<G::Vertex>::simple().weighted();
        tree.add_vertices(self.vertex_list.iter().cloned());
        for i in 1..self.n {
            let j = self.neighbors[i];
            tree.add_weighted_edge(
                &self.vertex_list[i],
                &self.vertex_list[j],
                self.flow_matrix[i][j],
            )?;
        }
        Ok(tree)
    }
}

impl<'g, G: Graph + 'g> MaximumFlowAlgorithm<G::Vertex, G::Edge>
    for GusfieldEquivalentFlowTree<'g, G>
{
    fn maximum_flow(
        &mut self,
        _source: &G::Vertex,
        _sink: &G::Vertex,
    ) -> Result<MaximumFlow<G::Edge>> {
        unsupported()
    }

    fn maximum_flow_value(&mut self, source: &G::Vertex, sink: &G::Vertex) -> Result<f64> {
        let (s, t) = match (self.index_map.get(source), self.index_map.get(sink)) {
            (Some(&s), Some(&t)) => (s, t),
            _ => return Err(GraphError::VertexNotFound),
        };
        if self.p.is_none() {
            self.calculate_equivalent_flow_tree()?;
        }
        Ok(self.flow_matrix[s][t])
    }

    fn flow_map(&mut self) -> Result<&HashMap<G::Edge, f64>> {
        unsupported()
    }

    fn flow_direction(&self, _edge: &G::Edge) -> Result<G::Vertex> {
        unsupported()
    }
}

/// `GusfieldGomoryHuCutTree`: a Gomory–Hu cut tree from `n - 1` minimum cut computations. Answers
/// maximum flow values and minimum cuts between any two vertices.
pub struct GusfieldGomoryHuCutTree<'g, G: Graph> {
    network: &'g G,
    n: usize,
    minimum_st_cut_algorithm: CutAlgorithm<'g, G>,
    vertex_list: Vec<G::Vertex>,
    index_map: HashMap<G::Vertex, usize>,
    p: Option<Vec<usize>>,
    fl: Vec<f64>,
    flow_matrix: Vec<Vec<f64>>,
    last_invoked_source: Option<G::Vertex>,
    last_invoked_target: Option<G::Vertex>,
    source_partition_last_invoked_source: Option<IndexSet<G::Vertex>>,
    gomory_hu_tree: Option<UGraph<G::Vertex>>,
    sink_cache: Option<IndexSet<G::Vertex>>,
    cut_cache: Option<IndexSet<G::Edge>>,
}

impl<'g, G: Graph + 'g> GusfieldGomoryHuCutTree<'g, G> {
    pub fn new(network: &'g G) -> Result<Self> {
        Self::with_epsilon(network, DEFAULT_EPSILON)
    }

    pub fn with_epsilon(network: &'g G, epsilon: f64) -> Result<Self> {
        Self::with_algorithm(
            network,
            Box::new(PushRelabelMFImpl::with_epsilon(network, epsilon)?),
        )
    }

    /// Uses `minimum_st_cut_algorithm` for the cut computations.
    pub fn with_algorithm(
        network: &'g G,
        minimum_st_cut_algorithm: CutAlgorithm<'g, G>,
    ) -> Result<Self> {
        require_undirected(network)?;
        let (vertex_list, index_map) = index_vertices(network)?;
        Ok(GusfieldGomoryHuCutTree {
            network,
            n: vertex_list.len(),
            minimum_st_cut_algorithm,
            vertex_list,
            index_map,
            p: None,
            fl: Vec::new(),
            flow_matrix: Vec::new(),
            last_invoked_source: None,
            last_invoked_target: None,
            source_partition_last_invoked_source: None,
            gomory_hu_tree: None,
            sink_cache: None,
            cut_cache: None,
        })
    }

    fn calculate_gomory_hu_tree(&mut self) -> Result<()> {
        let n = self.n;
        let mut flow_matrix = vec![vec![0.0; n]; n];
        let mut p = vec![0usize; n];
        let mut fl = vec![0.0; n];
        for s in 1..n {
            let t = p[s];
            let flow_value = self
                .minimum_st_cut_algorithm
                .calculate_min_cut(&self.vertex_list[s], &self.vertex_list[t])?;
            let source_partition = self.minimum_st_cut_algorithm.source_partition();
            fl[s] = flow_value;
            for i in 0..n {
                if i != s && source_partition.contains(&self.vertex_list[i]) && p[i] == t {
                    p[i] = s;
                }
            }
            if source_partition.contains(&self.vertex_list[p[t]]) {
                p[s] = p[t];
                p[t] = s;
                fl[s] = fl[t];
                fl[t] = flow_value;
            }
            flow_matrix[s][t] = flow_value;
            flow_matrix[t][s] = flow_value;
            for i in 0..s {
                if i != t {
                    let v = flow_matrix[s][t].min(flow_matrix[t][i]);
                    flow_matrix[s][i] = v;
                    flow_matrix[i][s] = v;
                }
            }
        }
        self.flow_matrix = flow_matrix;
        self.p = Some(p);
        self.fl = fl;
        Ok(())
    }

    /// The Gomory–Hu tree: an edge `(i, p(i))` weighted with the cut value `fl(i)`.
    pub fn gomory_hu_tree(&mut self) -> Result<UGraph<G::Vertex>> {
        if self.p.is_none() {
            self.calculate_gomory_hu_tree()?;
        }
        let p = self.p.as_ref().unwrap();
        let mut tree = UGraph::<G::Vertex>::simple().weighted();
        tree.add_vertices(self.vertex_list.iter().cloned());
        for i in 1..self.n {
            tree.add_weighted_edge(&self.vertex_list[i], &self.vertex_list[p[i]], self.fl[i])?;
        }
        Ok(tree)
    }

    /// The weight of a global minimum cut: the cheapest tree edge. Afterwards the source and
    /// sink are that edge's endpoints.
    pub fn calculate_global_min_cut(&mut self) -> Result<f64> {
        if self.gomory_hu_tree.is_none() {
            self.gomory_hu_tree = Some(self.gomory_hu_tree()?);
        }
        let tree = self.gomory_hu_tree.as_ref().unwrap();
        let mut cheapest: Option<(&DefaultEdge, f64)> = None;
        for e in tree.edges() {
            let w = tree.edge_weight(e).unwrap();
            if cheapest.is_none_or(|(_, best)| w < best) {
                cheapest = Some((e, w));
            }
        }
        let (edge, weight) =
            cheapest.ok_or_else(|| GraphError::InvalidArgument("graph is empty?!".into()))?;
        let (u, v) = tree.edge_endpoints(edge).unwrap();
        self.last_invoked_source = Some(u.clone());
        self.last_invoked_target = Some(v.clone());
        self.source_partition_last_invoked_source = None;
        Ok(weight)
    }

    fn find_path_between(
        &self,
        tree: &UGraph<G::Vertex>,
        source: &G::Vertex,
        target: &G::Vertex,
    ) -> Vec<DefaultEdge> {
        let mut visited = vec![false; self.vertex_list.len()];
        let mut predecessor: HashMap<G::Vertex, G::Vertex> = HashMap::new();
        let mut queue = VecDeque::from([source.clone()]);
        let mut found = false;
        while !found {
            let Some(next) = queue.pop_front() else { break };
            for v in tree.neighbors_of(&next) {
                if !visited[self.index_map[v]] {
                    predecessor.insert(v.clone(), next.clone());
                    queue.push_back(v.clone());
                }
                if v == target {
                    found = true;
                    break;
                }
            }
            visited[self.index_map[&next]] = true;
        }
        let mut edges = IndexSet::new();
        let mut v = target.clone();
        while v != *source {
            let pred = predecessor[&v].clone();
            edges.insert(tree.get_edge(&v, &pred).unwrap().clone());
            v = pred;
        }
        edges.into_iter().collect()
    }
}

impl<'g, G: Graph + 'g> MaximumFlowAlgorithm<G::Vertex, G::Edge>
    for GusfieldGomoryHuCutTree<'g, G>
{
    fn maximum_flow(
        &mut self,
        _source: &G::Vertex,
        _sink: &G::Vertex,
    ) -> Result<MaximumFlow<G::Edge>> {
        unsupported()
    }

    fn maximum_flow_value(&mut self, source: &G::Vertex, sink: &G::Vertex) -> Result<f64> {
        let (s, t) = match (self.index_map.get(source), self.index_map.get(sink)) {
            (Some(&s), Some(&t)) => (s, t),
            _ => return Err(GraphError::VertexNotFound),
        };
        self.last_invoked_source = Some(source.clone());
        self.last_invoked_target = Some(sink.clone());
        self.source_partition_last_invoked_source = None;
        self.gomory_hu_tree = None;
        if self.p.is_none() {
            self.calculate_gomory_hu_tree()?;
        }
        Ok(self.flow_matrix[s][t])
    }

    fn flow_map(&mut self) -> Result<&HashMap<G::Edge, f64>> {
        unsupported()
    }

    fn flow_direction(&self, _edge: &G::Edge) -> Result<G::Vertex> {
        unsupported()
    }
}

impl<'g, G: Graph + 'g> MinimumSTCutAlgorithm<G::Vertex, G::Edge>
    for GusfieldGomoryHuCutTree<'g, G>
{
    fn calculate_min_cut(&mut self, source: &G::Vertex, sink: &G::Vertex) -> Result<f64> {
        self.maximum_flow_value(source, sink)
    }

    fn cut_capacity(&mut self) -> f64 {
        let (s, t) = (
            self.last_invoked_source
                .clone()
                .expect("no cut computed yet"),
            self.last_invoked_target
                .clone()
                .expect("no cut computed yet"),
        );
        self.calculate_min_cut(&s, &t)
            .expect("vertices of the network")
    }

    fn source_partition(&mut self) -> &IndexSet<G::Vertex> {
        if self.source_partition_last_invoked_source.is_none() {
            if self.gomory_hu_tree.is_none() {
                self.gomory_hu_tree = Some(self.gomory_hu_tree().expect("tree"));
            }
            let source = self
                .last_invoked_source
                .clone()
                .expect("no cut computed yet");
            let target = self
                .last_invoked_target
                .clone()
                .expect("no cut computed yet");
            let path_edges = {
                let tree = self.gomory_hu_tree.as_ref().unwrap();
                self.find_path_between(tree, &source, &target)
            };
            let tree = self.gomory_hu_tree.as_mut().unwrap();
            let mut cheapest: Option<(DefaultEdge, f64)> = None;
            for e in &path_edges {
                let w = tree.edge_weight(e).unwrap();
                if cheapest.is_none_or(|(_, best)| w < best) {
                    cheapest = Some((*e, w));
                }
            }
            let (edge, weight) = cheapest.expect("path is empty?!");
            let (u, v) = {
                let (u, v) = tree.edge_endpoints(&edge).unwrap();
                (u.clone(), v.clone())
            };
            tree.remove_edge(&edge);
            let component = crate::alg::connectivity::ConnectivityInspector::new()
                .connected_set_of(&*tree, &source)
                .expect("source is in the tree");
            let partition = self
                .vertex_list
                .iter()
                .filter(|x| component.contains(*x))
                .cloned()
                .collect();
            tree.add_edge_with(&u, &v, edge)
                .expect("re-adding the removed edge");
            tree.set_edge_weight(&edge, weight).expect("weighted tree");
            self.source_partition_last_invoked_source = Some(partition);
        }
        self.source_partition_last_invoked_source.as_ref().unwrap()
    }

    fn sink_partition(&mut self) -> &IndexSet<G::Vertex> {
        let source_side = self.source_partition().clone();
        // Java builds a fresh set on every call; the field only lends it out
        let sink: IndexSet<G::Vertex> = self
            .network
            .vertices()
            .filter(|v| !source_side.contains(*v))
            .cloned()
            .collect();
        self.sink_cache.insert(sink)
    }

    fn cut_edges(&mut self) -> &IndexSet<G::Edge> {
        let source_side = self.source_partition().clone();
        let g = self.network;
        let cut: IndexSet<G::Edge> = g
            .edges()
            .filter(|e| {
                let (u, v) = g.edge_endpoints(e).unwrap();
                source_side.contains(u) ^ source_side.contains(v)
            })
            .cloned()
            .collect();
        self.cut_cache.insert(cut)
    }
}
