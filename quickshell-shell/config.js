function parse(text) {
    const data = JSON.parse(text);
    if (!data || !Array.isArray(data.surfaces)) throw Error('surfaces must be an array');
    const ids = {};
    const allowed = ['id','page','layer','anchors','width','height','exclusiveZone','margins','keyboardInteractivity','output','visible'];
    return data.surfaces.map(function(s) {
        if (!s || typeof s !== 'object' || Array.isArray(s)) throw Error('surface must be an object');
        Object.keys(s).forEach(function(k) { if (allowed.indexOf(k) < 0) throw Error('unknown field: '+k); });
        if (typeof s.id !== 'string' || !/^[a-zA-Z0-9_-]+$/.test(s.id) || ids[s.id]) throw Error('invalid or duplicate id');
        ids[s.id] = true;
        if (typeof s.page !== 'string' || !/^pages\/[A-Za-z0-9_-]+\.qml$/.test(s.page)) throw Error('page must be a local pages/*.qml component');
        const r = Object.assign({layer:'top', anchors:[], width:0, height:0, exclusiveZone:0, margins:{}, keyboardInteractivity:'none', output:'*', visible:true}, s);
        if (['background','bottom','top','overlay'].indexOf(r.layer)<0) throw Error('invalid layer');
        if (['none','exclusive','on-demand'].indexOf(r.keyboardInteractivity)<0) throw Error('invalid keyboard interactivity');
        if (!Array.isArray(r.anchors) || r.anchors.some(function(a) { return ['top','bottom','left','right'].indexOf(a)<0; }) || new Set(r.anchors).size !== r.anchors.length) throw Error('invalid anchors');
        ['width','height','exclusiveZone'].forEach(function(k) { if (!Number.isInteger(r[k]) || r[k] < (k==='exclusiveZone'?-1:0) || r[k]>16384) throw Error('invalid '+k); });
        if (!r.width && !(r.anchors.indexOf('left')>=0 && r.anchors.indexOf('right')>=0)) throw Error('width or opposing anchors required');
        if (!r.height && !(r.anchors.indexOf('top')>=0 && r.anchors.indexOf('bottom')>=0)) throw Error('height or opposing anchors required');
        if (!r.margins || typeof r.margins!=='object' || Array.isArray(r.margins)) throw Error('invalid margins');
        Object.keys(r.margins).forEach(function(k) { if (['top','bottom','left','right'].indexOf(k)<0 || !Number.isInteger(r.margins[k]) || Math.abs(r.margins[k])>16384) throw Error('invalid margin'); });
        if (typeof r.output!=='string' || !r.output || typeof r.visible!=='boolean') throw Error('invalid output or visible');
        return r;
    });
}
function expand(specs, screens) {
    const result = [];
    specs.forEach(function(s) { screens.forEach(function(screen) { if(s.output==='*' || s.output===screen.name) result.push({key:s.id+'@'+screen.name,spec:s,screen:screen}); }); });
    return result;
}
if (typeof module !== 'undefined') module.exports = {parse:parse, expand:expand};
