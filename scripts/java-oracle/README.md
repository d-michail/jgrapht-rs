# Java oracles

Small programs that run the real JGraphT (the sibling checkout, compiled to
`jgrapht-core/target/classes`, plus `jheaps`) on the graphs in
`crates/jgrapht/tests/alg/fixtures/ch_graphs.txt` and print the results that the Rust tests compare
with (`ch_golden.txt`, `tnr_golden.txt`).

The programs live in the `org.jgrapht.alg.shortestpath` package because they read package-private
fields:

```sh
CP=jgrapht-core/target/classes:$HOME/.m2/repository/org/jheaps/jheaps/0.15/jheaps-0.15.jar
mkdir -p oracle/org/jgrapht/alg/shortestpath out
cp ContractionHierarchyOracle.java oracle/org/jgrapht/alg/shortestpath/Oracle.java   # class Oracle
cp TransitNodeRoutingOracle.java   oracle/org/jgrapht/alg/shortestpath/TnrOracle.java # class TnrOracle
javac -cp $CP -d out oracle/org/jgrapht/alg/shortestpath/*.java
java -cp out:$CP org.jgrapht.alg.shortestpath.Oracle    ch_graphs.txt > ch_golden.txt
java -cp out:$CP org.jgrapht.alg.shortestpath.TnrOracle ch_graphs.txt > tnr_golden.txt
```
