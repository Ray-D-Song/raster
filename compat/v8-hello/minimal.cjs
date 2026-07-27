'use strict';
const path = require('node:path');
console.log('loading addon...');
const hello = require(path.join(__dirname, 'build/Release/v8_hello.node'));
console.log('loaded, calling hello()');
const v = hello.hello();
console.log('result:', v);
