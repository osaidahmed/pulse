int process_order(int status, int verified, int stock, int shipped, int cancelled, int delivered) {
    if (status == 0) {
        if (verified) {
            if (stock) {
                status = 1;
            } else {
                status = 2;
            }
        } else {
            status = 3;
        }
    } else if (status == 1) {
        if (shipped) {
            status = 4;
        } else if (cancelled) {
            status = 5;
        }
    } else if (status == 4) {
        if (delivered) {
            status = 6;
        }
    }
    return status;
}
