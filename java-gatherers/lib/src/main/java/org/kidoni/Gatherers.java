package org.kidoni;

import java.util.ArrayList;
import java.util.List;
import java.util.function.Predicate;
import java.util.stream.Gatherer;

@SuppressWarnings("preview")
public abstract class Gatherers {

    // see https://doc.rust-lang.org/std/primitive.slice.html#method.split
    public static <T> Gatherer<T, ?, List<T>> split(Predicate<? super T> predicate) {
        class SplitState {
            final List<T> list = new ArrayList<>();
            boolean did_push;
        }

        return Gatherer.<T, SplitState, List<T>>ofSequential(
                SplitState::new,
                (state, element, downstream) -> {
                    if (predicate.test(element)) {
                        List<T> copy = List.copyOf(state.list);
                        state.list.clear();
                        downstream.push(copy);
                        state.did_push = true;
                    }
                    else {
                        state.list.add(element);
                        state.did_push = false;
                    }

                    return true;
                },
                (state, downstream) -> {
                    if (state.did_push || !state.list.isEmpty()) {
                        downstream.push(List.copyOf(state.list));
                    }
                });
    }
}
