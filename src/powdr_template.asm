machine ZkML {

    // Default EZKL lookup range is (-2^15, 2^15-1), so we need tables of that size
    degree 65536;

    reg pc[@pc];
    reg X1[<=];
    reg X2[<=];
    reg X3[<=];
    reg Y[<=];

    let a;
    let b;
    let m;
    let res;

    // Write-once memory
    let ADDR = |i| i;
    let mem;

    // Base instructions
    instr mult X1, X2, Y -> {
        {X1, a} in {ADDR, mem},
        {X2, b} in {ADDR, mem},
        {Y, res} in {ADDR, mem},
        res = a * b
    }
    instr dot X1, X2, X3, Y -> {
        {X1, a} in {ADDR, mem},
        {X2, b} in {ADDR, mem},
        {X3, m} in {ADDR, mem},
        {Y, res} in {ADDR, mem},
        res = a * b + m
    }
    instr add X1, X2, Y -> {
        {X1, a} in {ADDR, mem},
        {X2, b} in {ADDR, mem},
        {Y, res} in {ADDR, mem},
        res = a + b
    }

    // div_128
    col fixed X(i) {i - 32768};
    col fixed DIV_128_Y(i) {
        match i < 32768 {
            // Implement division with rounding...
            1 => -(((32768 - i) / 128) + match (((32768 - i) % 128) < 64) {1 => 0, 0 => 1}),
            0 => (i - 32768) / 128 + match (((i - 32768) % 128) < 64) {1 => 0, 0 => 1}
        }
    };
    instr div_128 X1, Y -> {
        {X1, a} in {ADDR, mem},
        {Y, res} in {ADDR, mem},
        {a, res} in {X, DIV_128_Y}
    }

    // relu
    col fixed RELU_Y(i) {match i < 32768 {1 => 0, 0 => i - 32768}};
    instr relu X1, Y -> {
        {X1, a} in {ADDR, mem},
        {Y, res} in {ADDR, mem},
        {a, res} in {X, RELU_Y}
    }

    function main {

{{program}}

        return;
    }
}