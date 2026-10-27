package org.jgrapht.alg.shortestpath;

import org.jgrapht.*;
import org.jgrapht.graph.*;
import org.jgrapht.util.ConcurrencyUtil;
import java.io.*;
import java.util.*;
import java.util.concurrent.ThreadPoolExecutor;
import static org.jgrapht.alg.shortestpath.ContractionHierarchyPrecomputation.*;

public class Oracle {
    public static void main(String[] a) throws Exception {
        BufferedReader in = new BufferedReader(new FileReader(a[0]));
        int[] ps = {1, 3, 4};
        String line;
        while ((line = in.readLine()) != null) {
            String[] h = line.split(" ");
            boolean directed = h[2].equals("d");
            int n = Integer.parseInt(h[3]), m = Integer.parseInt(h[4]);
            int[][] es = new int[m][3];
            for (int i = 0; i < m; i++) {
                String[] t = in.readLine().split(" ");
                for (int j = 0; j < 3; j++) es[i][j] = Integer.parseInt(t[j]);
            }
            for (int p : ps) {
                Graph<Integer, DefaultWeightedEdge> g = directed
                    ? new DirectedWeightedPseudograph<>(DefaultWeightedEdge.class)
                    : new WeightedPseudograph<>(DefaultWeightedEdge.class);
                for (int i = 0; i < n; i++) g.addVertex(i);
                for (int[] e : es) { DefaultWeightedEdge x = g.addEdge(e[0], e[1]); g.setEdgeWeight(x, e[2]); }
                ThreadPoolExecutor ex = ConcurrencyUtil.createThreadPoolExecutor(p);
                ContractionHierarchy<Integer, DefaultWeightedEdge> ch =
                    new ContractionHierarchyPrecomputation<>(g, () -> new Random(19), ex).computeContractionHierarchy();
                System.out.println("case " + h[1] + " " + p);
                for (int v = 0; v < n; v++) {
                    ContractionVertex<Integer> cv = ch.getContractionMapping().get(v);
                    System.out.println("L " + v + " " + cv.contractionLevel + " " + cv.vertexId);
                }
                Graph<ContractionVertex<Integer>, ContractionEdge<DefaultWeightedEdge>> cg = ch.getContractionGraph();
                for (ContractionEdge<DefaultWeightedEdge> e : cg.edgeSet()) {
                    System.out.println("E " + cg.getEdgeSource(e).vertex + " " + cg.getEdgeTarget(e).vertex + " "
                        + cg.getEdgeWeight(e) + " " + e.originalEdges + " " + (e.edge == null ? 0 : 1) + " " + (e.isUpward ? 1 : 0));
                }
                ConcurrencyUtil.shutdownExecutionService(ex);
            }
        }
    }
}
