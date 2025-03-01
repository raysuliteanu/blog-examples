package org.kidoni;

import java.util.List;
import java.util.stream.Stream;
import org.junit.jupiter.api.Test;

import static org.junit.jupiter.api.Assertions.assertEquals;

class GathererTest {
    @Test
    void evenSplit() {
        var splitter = org.kidoni.Gatherers.split((e) -> e.equals(0));
        var result = Stream.of(1, 2, 0, 3, 4, 0, 5, 6)
                .gather(splitter)
                .toList();

        assertEquals(3, result.size());
        assertEquals(List.of(1, 2), result.get(0));
        assertEquals(List.of(3, 4), result.get(1));
        assertEquals(List.of(5, 6), result.get(2));
    }

    @Test
    void oddSplit() {
        var splitter = org.kidoni.Gatherers.split((e) -> e.equals(0));
        var result = Stream.of(1, 2, 0, 3, 4, 0, 5)
                .gather(splitter)
                .toList();

        assertEquals(3, result.size());
        assertEquals(List.of(1, 2), result.get(0));
        assertEquals(List.of(3, 4), result.get(1));
        assertEquals(List.of(5), result.get(2));
    }

    @Test
    void noLast() {
        var splitter = org.kidoni.Gatherers.split((e) -> e.equals(0));
        var result = Stream.of(1, 2, 0, 3, 4, 0)
                .gather(splitter)
                .toList();

        assertEquals(2, result.size());
        assertEquals(List.of(1, 2), result.get(0));
        assertEquals(List.of(3, 4), result.get(1));
    }
}
