use jgrapht::{DiGraph, Graph};
use jgrapht_io::dimacs::{DimacsExporter, DimacsFormat, DimacsImporter};
use jgrapht_io::{GraphExporter, GraphImporter, Result};

fn main() -> Result<()> {
    let input = "p sp 3 2\na 1 2 1.5\na 2 3 2.5\n";
    let mut graph = DiGraph::<usize>::multi()
        .weighted()
        .with_vertex_factory(|id| id as usize);
    DimacsImporter::new().import_str(&mut graph, input)?;

    let mut exporter =
        DimacsExporter::with_vertex_id_provider(|vertex: &usize| (vertex + 1).to_string());
    exporter.set_format(DimacsFormat::ShortestPath);
    print!("{}", exporter.export_to_string(&graph)?);
    assert_eq!(graph.edge_count(), 2);
    Ok(())
}
