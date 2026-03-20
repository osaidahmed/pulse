function deeplyNested(data) {
    for (const group of data) {
        if (group.active) {
            for (const item of group.items) {
                if (item.valid) {
                    for (const sub of item.children) {
                        if (sub.ready) {
                            process(sub);
                        }
                    }
                }
            }
        }
    }
    return true;
}

function moderatelyNested(data) {
    for (const item of data) {
        if (item.active) {
            for (const sub of item.children) {
                process(sub);
            }
        }
    }
    return true;
}

function process(x) {}
