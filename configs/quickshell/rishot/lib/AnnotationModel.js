var UNDO_LIMIT = 100;

function cloneItem(a) {
    var c = {};
    for (var k in a) c[k] = a[k];
    c.points = a.points.map(function (p) { return { x: p.x, y: p.y }; });
    return c;
}

function clone(items) {
    return items.map(cloneItem);
}

function create() {
    return {
        items: [],
        undoStack: [],
        redoStack: [],

        commit: function () {
            this.undoStack.push(clone(this.items));
            if (this.undoStack.length > UNDO_LIMIT) this.undoStack.shift();
            this.redoStack = [];
        },

        add: function (ann) {
            this.commit();
            this.items.push(ann);
            return ann;
        },

        move: function (index, dx, dy) {
            if (index < 0 || index >= this.items.length) return false;
            this.commit();
            var pts = this.items[index].points;
            for (var i = 0; i < pts.length; i++) {
                pts[i].x += dx;
                pts[i].y += dy;
            }
            return true;
        },

        remove: function (index) {
            if (index < 0 || index >= this.items.length) return false;
            this.commit();
            this.items.splice(index, 1);
            return true;
        },

        undo: function () {
            if (this.undoStack.length === 0) return false;
            this.redoStack.push(clone(this.items));
            this.items = this.undoStack.pop();
            return true;
        },

        redo: function () {
            if (this.redoStack.length === 0) return false;
            this.undoStack.push(clone(this.items));
            this.items = this.redoStack.pop();
            return true;
        },

        canUndo: function () { return this.undoStack.length > 0; },
        canRedo: function () { return this.redoStack.length > 0; }
    };
}

if (typeof module !== "undefined" && module.exports) {
    module.exports = { create: create };
}
