package org.kidoni;

import java.util.ArrayList;
import java.util.List;
import java.util.function.Predicate;
import java.util.stream.Gatherer;

public abstract class Gatherers {

    public static <T> Gatherer<T, List<T>, List<T>> split(Predicate<? super T> predicate) {
        return Gatherer.<T, List<T>, List<T>>ofSequential(
                ArrayList::new,
                (list, element, downstream) -> {
                    if (predicate.test(element)) {
                        List<T> copy = List.copyOf(list);
                        list.clear();
                        return downstream.push(copy);
                    }

                    list.add(element);
                    return true;
                },
                (elements, downstream) -> {
                    if (!elements.isEmpty()) {
                        downstream.push(List.copyOf(elements));
                    }
                });
    }
}
