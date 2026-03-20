import java.util.List;

public class DeepNesting {
    public boolean deeplyNested(List<List<List<Integer>>> data) {
        for (var group : data) {
            if (!group.isEmpty()) {
                for (var items : group) {
                    if (!items.isEmpty()) {
                        for (var val : items) {
                            if (val > 0) {
                                process(val);
                            }
                        }
                    }
                }
            }
        }
        return true;
    }

    public boolean moderatelyNested(List<Integer> data) {
        for (var item : data) {
            if (item > 0) {
                process(item);
            }
        }
        return true;
    }

    private void process(int x) {}
}
