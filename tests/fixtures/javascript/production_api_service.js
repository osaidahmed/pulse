function dispatchCommand(cmd) {
    switch (cmd) {
    case "start":
        return "starting";
    case "stop":
        return "stopping";
    case "restart":
        return "restarting";
    case "status":
        return "checking";
    case "deploy":
        return "deploying";
    case "rollback":
        return "rolling back";
    }
    return "unknown";
}

function processEvent(data) {
    let a = data[0];
    let b = data[1];
    let c = data[2];
    let d = data[3];
    let e = data[4];
    let f = data[5];
    let g = data[6];
    let h = data[7];
    if (a > 100) {
        return -1;
    }
    if (b > 200) {
        return -2;
    }
    const result = a + b + c + d;
    const extra = e + f + g + h;
    return result + extra;
}
