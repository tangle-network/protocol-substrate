pragma circom 2.0.0;

/*This circuit template checks that c is the multiplication of a and b.*/  

template Test() {  
   // Declaration of signals.  
   signal input a; 
   signal input b;  
   signal input c;  
   signal output out;  

   // Constraints.  
   c === a * b;
   out <== c + a;
}

component main = Test();
