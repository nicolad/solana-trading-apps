export interface Env {
  LASER: any;
  HELIUS_API_KEY: string;
  LASERSTREAM_ENDPOINT: string;
}

export class LaserContainer {
  state: any;
  env: Env;

  constructor(state: any, env: Env) {
    this.state = state;
    this.env = env;
  }

  async fetch(request: Request): Promise<Response> {
    return new Response(
      JSON.stringify({
        status: "Container-backed Durable Object",
        endpoint: this.env.LASERSTREAM_ENDPOINT,
        path: new URL(request.url).pathname,
      }),
      {
        headers: { "content-type": "application/json" },
      }
    );
  }
}

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const id = env.LASER.idFromName("main");
    const stub = env.LASER.get(id);
    return stub.fetch(request);
  },
};
