package org.jgrapht.alg.shortestpath;

import org.jgrapht.*;
import org.jgrapht.graph.*;
import org.jgrapht.util.ConcurrencyUtil;
import java.io.*;
import java.util.*;
import java.util.concurrent.ThreadPoolExecutor;
import static org.jgrapht.alg.shortestpath.ContractionHierarchyPrecomputation.*;
import static org.jgrapht.alg.shortestpath.TransitNodeRoutingPrecomputation.*;

public class TnrOracle {
    public static void main(String[] a) throws Exception {
        BufferedReader in = new BufferedReader(new FileReader(a[0]));
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
            int p = 3;
            Graph<Integer, DefaultWeightedEdge> g = directed
                ? new DirectedWeightedPseudograph<>(DefaultWeightedEdge.class)
                : new WeightedPseudograph<>(DefaultWeightedEdge.class);
            for (int i = 0; i < n; i++) g.addVertex(i);
            for (int[] e : es) { DefaultWeightedEdge x = g.addEdge(e[0], e[1]); g.setEdgeWeight(x, e[2]); }
            ThreadPoolExecutor ex = ConcurrencyUtil.createThreadPoolExecutor(p);
            ContractionHierarchy<Integer, DefaultWeightedEdge> ch =
                new ContractionHierarchyPrecomputation<>(g, () -> new Random(19), ex).computeContractionHierarchy();
            TransitNodeRouting<Integer, DefaultWeightedEdge> r =
                new TransitNodeRoutingPrecomputation<>(ch, ex).computeTransitNodeRouting();
            System.out.println("case " + h[1]);
            TreeSet<Integer> transit = new TreeSet<>();
            for (ContractionVertex<Integer> v : r.getTransitVertices()) transit.add(v.vertex);
            System.out.println("T " + transit);
            for (int v = 0; v < n; v++) {
                ContractionVertex<Integer> cv = ch.getContractionMapping().get(v);
                TreeSet<Integer> fa = new TreeSet<>(), ba = new TreeSet<>();
                for (AccessVertex<Integer, DefaultWeightedEdge> x : r.getAccessVertices().getForwardAccessVertices(cv)) fa.add(x.getVertex());
                for (AccessVertex<Integer, DefaultWeightedEdge> x : r.getAccessVertices().getBackwardAccessVertices(cv)) ba.add(x.getVertex());
                System.out.println("V " + v + " " + r.getVoronoiDiagram().getVoronoiCellId(cv) + " F" + fa + " B" + ba);
            }
            StringBuilder sb = new StringBuilder("LF ");
            for (int s = 0; s < n; s++) for (int t = 0; t < n; t++) sb.append(r.getLocalityFilter().isLocal(s, t) ? '1' : '0');
            System.out.println(sb);
            ConcurrencyUtil.shutdownExecutionService(ex);
        }
    }
}
