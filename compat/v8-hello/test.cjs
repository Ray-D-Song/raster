'use strict';

const path = require('node:path');
const hello = require('./build/Release/v8_hello.node');

const value = hello.hello();
if (typeof value !== 'string' || value !== 'hello from v8') {
  console.error('unexpected hello():', value);
  process.exit(1);
}

console.log('v8-hello compat OK');
