package org.kidoni;

import java.util.ArrayList;
import java.util.List;
import java.util.function.Predicate;
import java.util.stream.Gatherer;

@SuppressWarnings("preview")
public abstract class Gatherers {

    ///  Implement a function similar to the split() method in Rust on e.g. a slice.
    /// It is not exactly the same, since the result of split() in Rust is an iterator.
    /// With Java Stream API you'd need to explicitly call
    /// [Stream.iterator()](https://docs.oracle.com/en/java/javase/23/docs/api/java.base/java/util/stream/BaseStream.html#iterator())
    ///
    /// See Rust [std::slice::Split](https://doc.rust-lang.org/std/slice/struct.Split.html)
    public static <T> Gatherer<T, ?, List<T>> split(Predicate<? super T> predicate) {
        class SplitState {
            final List<T> list = new ArrayList<>();
            boolean did_push;
        }

        return Gatherer.ofSequential(
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

    ///  Implement a function similar to the split() method in Rust on e.g. a slice.
    /// See [std::slice::splitn](https://doc.rust-lang.org/std/primitive.slice.html#method.splitn)
    public static <T> Gatherer<T, ?, List<T>> splitn(int n, Predicate<? super T> predicate) {
        class SplitState {
            final List<T> list = new ArrayList<>();
            boolean did_push;
            int splits = 1;
        }

        if (n < 1) {
            throw new IllegalArgumentException("n must be greater than 0");
        }

        return Gatherer.ofSequential(
                SplitState::new,
                (state, element, downstream) -> {
                    if (state.splits < n && predicate.test(element)) {
                        List<T> copy = List.copyOf(state.list);
                        state.list.clear();
                        downstream.push(copy);
                        state.did_push = true;
                        state.splits++;
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
