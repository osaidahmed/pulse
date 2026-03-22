<?php

function test_user_validation(): void {
    assert(validate("a") === true);
    assert(validate("b") === true);
    assert(validate("c") === true);
    assert(validate("d") === true);
    assert(validate("e") === true);
    assert(validate("f") === true);
    assert(validate("g") === true);
    assert(validate("h") === true);
    assert(validate("i") === true);
    assert(validate("j") === true);
    assert(validate("k") === true);
    assert(validate("l") === true);
}
