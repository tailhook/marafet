define('render', ['exports'], function(exports) {
    var registrations = [];

    exports.append = function(el, fun) {
        var node = cito.vdom.append(el, fun);
        registrations.push({
            node: node,
            renderer: fun,
            });
    }

    exports.update = function() {
        for(var i = 0, il = registrations.length; i < il; ++i) {
            var ob = registrations[i];
            cito.vdom.update(ob.node, ob.renderer);
        }
    }
})

define('streams', ['exports', 'render'], function(exports, render) {
    function Stream(name) {
        this.name = name;
        this.handle_event = this.handle_event.bind(this)
        this._handlers = []
    }

    Stream.prototype.handle_event = function(ev) {
        console.log("EVENT", this.name, ev, this._handlers)
        var h = this._handlers;
        for(var i = 0, li = h.length; i < li; ++i) {
            try {
                h[i](ev)
            } catch(e) {
                console.error("Error handing event", ev,
                              "in stream", this.name, e)
            }
        }
        render.update();
    }
    Stream.prototype.handle = function(fun) {
        this._handlers.push(fun);
    }


    exports.Stream = Stream
})

define('stores', ['exports', 'streams'], function(exports, streams) {
    var Stream = streams.Stream

    function Tooltip() {
        this.mouseenter = new Stream('tooltip_hover')
        this.mouseleave = new Stream('tooltip_leave')
        this.mouseenter.handle(this.show.bind(this))
        this.mouseleave.handle(this.hide.bind(this))
        this.visible = false
    }

    Tooltip.prototype.show = function(ev) {
        this.x = ev.pageX
        this.y = ev.pageY
        this.visible = true
    }

    Tooltip.prototype.hide = function() {
        this.visible = false
    }

    Tooltip.prototype.style = function() {
        return {
            position: 'fixed',
            left: this.x + 'px',
            top: this.y + 'px',
            }
    }

    function Toggle() {
        this.toggle = new Stream('toggle_event')
        this.toggle.handle(this.do_toggle.bind(this))
        this.visible = false
    }

    Toggle.prototype.do_toggle = function() {
        this.visible = !this.visible;
    }

    exports.Tooltip = Tooltip
    exports.Toggle = Toggle
})
