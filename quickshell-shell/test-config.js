const assert = require('node:assert/strict');
const fs = require('node:fs');
const {parse, expand} = require('./config.js');
const text = fs.readFileSync('surfaces.json','utf8');
const specs = parse(text);
assert.equal(specs.length,4);
assert.deepEqual(expand([specs[0]], [{name:'eDP-1'},{name:'DP-3'}]).map(x=>x.key), ['bar@eDP-1','bar@DP-3']);
assert.equal(expand([{...specs[0],output:'missing'}],[{name:'eDP-1'}]).length,0);
for(const change of [{id:'bad@id'},{page:'https://example.com/x.qml'},{width:-1},{layer:'invalid'},{anchors:['top','top']},{visible:'yes'},{margins:[]},{typo:1}]) {
    assert.throws(()=>parse(JSON.stringify({surfaces:[{...specs[0],...change}]})));
}
assert.throws(()=>parse('{'));
assert.throws(()=>parse(JSON.stringify({surfaces:[specs[0],specs[0]]})));
console.log('PASS configuration validation and output expansion');
