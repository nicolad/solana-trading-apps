export class LaserContainer {
    state;
    env;
    constructor(state, env) {
        this.state = state;
        this.env = env;
    }
    async fetch(request) {
        return new Response(JSON.stringify({
            status: "Container-backed Durable Object",
            endpoint: this.env.LASERSTREAM_ENDPOINT,
            path: new URL(request.url).pathname,
        }), {
            headers: { "content-type": "application/json" },
        });
    }
}
export default {
    async fetch(request, env) {
        const id = env.LASER.idFromName("main");
        const stub = env.LASER.get(id);
        return stub.fetch(request);
    },
};
