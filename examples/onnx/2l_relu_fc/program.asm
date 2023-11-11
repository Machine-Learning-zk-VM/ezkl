// Row 3: mult
a <== mload(0);
b <== mload(1);
m <== mult();
mstore(2);
// Row 4: dot
a <== mload(3);
b <== mload(4);
m <== dot();
mstore(5);
// Row 5: dot
a <== mload(6);
b <== mload(7);
m <== dot();
mstore(8);
// Row 6: div_128
a <== mload(8);
m <== div_128();
mstore(9);
// Row 7: add
a <== mload(9);
b <== mload(10);
m <== add();
mstore(11);
// Row 8: relu
a <== mload(11);
m <== relu();
mstore(12);
