var __defProp = Object.defineProperty;
var __defNormalProp = (obj, key, value) => key in obj ? __defProp(obj, key, { enumerable: true, configurable: true, writable: true, value }) : obj[key] = value;
var __name = (target, value) => __defProp(target, "name", { value, configurable: true });
var __publicField = (obj, key, value) => {
  __defNormalProp(obj, typeof key !== "symbol" ? key + "" : key, value);
  return value;
};

// .wrangler/tmp/bundle-fCESHO/strip-cf-connecting-ip-header.js
function stripCfConnectingIPHeader(input, init) {
  const request = new Request(input, init);
  request.headers.delete("CF-Connecting-IP");
  return request;
}
__name(stripCfConnectingIPHeader, "stripCfConnectingIPHeader");
globalThis.fetch = new Proxy(globalThis.fetch, {
  apply(target, thisArg, argArray) {
    return Reflect.apply(target, thisArg, [
      stripCfConnectingIPHeader.apply(null, argArray)
    ]);
  }
});

// node_modules/hono/dist/compose.js
var compose = /* @__PURE__ */ __name((middleware, onError, onNotFound) => {
  return (context, next) => {
    let index = -1;
    return dispatch(0);
    async function dispatch(i) {
      if (i <= index) {
        throw new Error("next() called multiple times");
      }
      index = i;
      let res;
      let isError = false;
      let handler;
      if (middleware[i]) {
        handler = middleware[i][0][0];
        context.req.routeIndex = i;
      } else {
        handler = i === middleware.length && next || void 0;
      }
      if (handler) {
        try {
          res = await handler(context, () => dispatch(i + 1));
        } catch (err) {
          if (err instanceof Error && onError) {
            context.error = err;
            res = await onError(err, context);
            isError = true;
          } else {
            throw err;
          }
        }
      } else {
        if (context.finalized === false && onNotFound) {
          res = await onNotFound(context);
        }
      }
      if (res && (context.finalized === false || isError)) {
        context.res = res;
      }
      return context;
    }
    __name(dispatch, "dispatch");
  };
}, "compose");

// node_modules/hono/dist/request/constants.js
var GET_MATCH_RESULT = /* @__PURE__ */ Symbol();

// node_modules/hono/dist/utils/body.js
var parseBody = /* @__PURE__ */ __name(async (request, options = /* @__PURE__ */ Object.create(null)) => {
  const { all = false, dot = false } = options;
  const headers = request instanceof HonoRequest ? request.raw.headers : request.headers;
  const contentType = headers.get("Content-Type");
  if (contentType?.startsWith("multipart/form-data") || contentType?.startsWith("application/x-www-form-urlencoded")) {
    return parseFormData(request, { all, dot });
  }
  return {};
}, "parseBody");
async function parseFormData(request, options) {
  const formData = await request.formData();
  if (formData) {
    return convertFormDataToBodyData(formData, options);
  }
  return {};
}
__name(parseFormData, "parseFormData");
function convertFormDataToBodyData(formData, options) {
  const form = /* @__PURE__ */ Object.create(null);
  formData.forEach((value, key) => {
    const shouldParseAllValues = options.all || key.endsWith("[]");
    if (!shouldParseAllValues) {
      form[key] = value;
    } else {
      handleParsingAllValues(form, key, value);
    }
  });
  if (options.dot) {
    Object.entries(form).forEach(([key, value]) => {
      const shouldParseDotValues = key.includes(".");
      if (shouldParseDotValues) {
        handleParsingNestedValues(form, key, value);
        delete form[key];
      }
    });
  }
  return form;
}
__name(convertFormDataToBodyData, "convertFormDataToBodyData");
var handleParsingAllValues = /* @__PURE__ */ __name((form, key, value) => {
  if (form[key] !== void 0) {
    if (Array.isArray(form[key])) {
      ;
      form[key].push(value);
    } else {
      form[key] = [form[key], value];
    }
  } else {
    if (!key.endsWith("[]")) {
      form[key] = value;
    } else {
      form[key] = [value];
    }
  }
}, "handleParsingAllValues");
var handleParsingNestedValues = /* @__PURE__ */ __name((form, key, value) => {
  let nestedForm = form;
  const keys = key.split(".");
  keys.forEach((key2, index) => {
    if (index === keys.length - 1) {
      nestedForm[key2] = value;
    } else {
      if (!nestedForm[key2] || typeof nestedForm[key2] !== "object" || Array.isArray(nestedForm[key2]) || nestedForm[key2] instanceof File) {
        nestedForm[key2] = /* @__PURE__ */ Object.create(null);
      }
      nestedForm = nestedForm[key2];
    }
  });
}, "handleParsingNestedValues");

// node_modules/hono/dist/utils/url.js
var splitPath = /* @__PURE__ */ __name((path) => {
  const paths = path.split("/");
  if (paths[0] === "") {
    paths.shift();
  }
  return paths;
}, "splitPath");
var splitRoutingPath = /* @__PURE__ */ __name((routePath) => {
  const { groups, path } = extractGroupsFromPath(routePath);
  const paths = splitPath(path);
  return replaceGroupMarks(paths, groups);
}, "splitRoutingPath");
var extractGroupsFromPath = /* @__PURE__ */ __name((path) => {
  const groups = [];
  path = path.replace(/\{[^}]+\}/g, (match2, index) => {
    const mark = `@${index}`;
    groups.push([mark, match2]);
    return mark;
  });
  return { groups, path };
}, "extractGroupsFromPath");
var replaceGroupMarks = /* @__PURE__ */ __name((paths, groups) => {
  for (let i = groups.length - 1; i >= 0; i--) {
    const [mark] = groups[i];
    for (let j = paths.length - 1; j >= 0; j--) {
      if (paths[j].includes(mark)) {
        paths[j] = paths[j].replace(mark, groups[i][1]);
        break;
      }
    }
  }
  return paths;
}, "replaceGroupMarks");
var patternCache = {};
var getPattern = /* @__PURE__ */ __name((label, next) => {
  if (label === "*") {
    return "*";
  }
  const match2 = label.match(/^\:([^\{\}]+)(?:\{(.+)\})?$/);
  if (match2) {
    const cacheKey = `${label}#${next}`;
    if (!patternCache[cacheKey]) {
      if (match2[2]) {
        patternCache[cacheKey] = next && next[0] !== ":" && next[0] !== "*" ? [cacheKey, match2[1], new RegExp(`^${match2[2]}(?=/${next})`)] : [label, match2[1], new RegExp(`^${match2[2]}$`)];
      } else {
        patternCache[cacheKey] = [label, match2[1], true];
      }
    }
    return patternCache[cacheKey];
  }
  return null;
}, "getPattern");
var tryDecode = /* @__PURE__ */ __name((str, decoder2) => {
  try {
    return decoder2(str);
  } catch {
    return str.replace(/(?:%[0-9A-Fa-f]{2})+/g, (match2) => {
      try {
        return decoder2(match2);
      } catch {
        return match2;
      }
    });
  }
}, "tryDecode");
var tryDecodeURI = /* @__PURE__ */ __name((str) => tryDecode(str, decodeURI), "tryDecodeURI");
var getPath = /* @__PURE__ */ __name((request) => {
  const url = request.url;
  const start = url.indexOf("/", url.indexOf(":") + 4);
  let i = start;
  for (; i < url.length; i++) {
    const charCode = url.charCodeAt(i);
    if (charCode === 37) {
      const queryIndex = url.indexOf("?", i);
      const path = url.slice(start, queryIndex === -1 ? void 0 : queryIndex);
      return tryDecodeURI(path.includes("%25") ? path.replace(/%25/g, "%2525") : path);
    } else if (charCode === 63) {
      break;
    }
  }
  return url.slice(start, i);
}, "getPath");
var getPathNoStrict = /* @__PURE__ */ __name((request) => {
  const result = getPath(request);
  return result.length > 1 && result.at(-1) === "/" ? result.slice(0, -1) : result;
}, "getPathNoStrict");
var mergePath = /* @__PURE__ */ __name((base, sub, ...rest) => {
  if (rest.length) {
    sub = mergePath(sub, ...rest);
  }
  return `${base?.[0] === "/" ? "" : "/"}${base}${sub === "/" ? "" : `${base?.at(-1) === "/" ? "" : "/"}${sub?.[0] === "/" ? sub.slice(1) : sub}`}`;
}, "mergePath");
var checkOptionalParameter = /* @__PURE__ */ __name((path) => {
  if (path.charCodeAt(path.length - 1) !== 63 || !path.includes(":")) {
    return null;
  }
  const segments = path.split("/");
  const results = [];
  let basePath = "";
  segments.forEach((segment) => {
    if (segment !== "" && !/\:/.test(segment)) {
      basePath += "/" + segment;
    } else if (/\:/.test(segment)) {
      if (/\?/.test(segment)) {
        if (results.length === 0 && basePath === "") {
          results.push("/");
        } else {
          results.push(basePath);
        }
        const optionalSegment = segment.replace("?", "");
        basePath += "/" + optionalSegment;
        results.push(basePath);
      } else {
        basePath += "/" + segment;
      }
    }
  });
  return results.filter((v, i, a) => a.indexOf(v) === i);
}, "checkOptionalParameter");
var _decodeURI = /* @__PURE__ */ __name((value) => {
  if (!/[%+]/.test(value)) {
    return value;
  }
  if (value.indexOf("+") !== -1) {
    value = value.replace(/\+/g, " ");
  }
  return value.indexOf("%") !== -1 ? tryDecode(value, decodeURIComponent_) : value;
}, "_decodeURI");
var _getQueryParam = /* @__PURE__ */ __name((url, key, multiple) => {
  let encoded;
  if (!multiple && key && !/[%+]/.test(key)) {
    let keyIndex2 = url.indexOf("?", 8);
    if (keyIndex2 === -1) {
      return void 0;
    }
    if (!url.startsWith(key, keyIndex2 + 1)) {
      keyIndex2 = url.indexOf(`&${key}`, keyIndex2 + 1);
    }
    while (keyIndex2 !== -1) {
      const trailingKeyCode = url.charCodeAt(keyIndex2 + key.length + 1);
      if (trailingKeyCode === 61) {
        const valueIndex = keyIndex2 + key.length + 2;
        const endIndex = url.indexOf("&", valueIndex);
        return _decodeURI(url.slice(valueIndex, endIndex === -1 ? void 0 : endIndex));
      } else if (trailingKeyCode == 38 || isNaN(trailingKeyCode)) {
        return "";
      }
      keyIndex2 = url.indexOf(`&${key}`, keyIndex2 + 1);
    }
    encoded = /[%+]/.test(url);
    if (!encoded) {
      return void 0;
    }
  }
  const results = {};
  encoded ??= /[%+]/.test(url);
  let keyIndex = url.indexOf("?", 8);
  while (keyIndex !== -1) {
    const nextKeyIndex = url.indexOf("&", keyIndex + 1);
    let valueIndex = url.indexOf("=", keyIndex);
    if (valueIndex > nextKeyIndex && nextKeyIndex !== -1) {
      valueIndex = -1;
    }
    let name = url.slice(
      keyIndex + 1,
      valueIndex === -1 ? nextKeyIndex === -1 ? void 0 : nextKeyIndex : valueIndex
    );
    if (encoded) {
      name = _decodeURI(name);
    }
    keyIndex = nextKeyIndex;
    if (name === "") {
      continue;
    }
    let value;
    if (valueIndex === -1) {
      value = "";
    } else {
      value = url.slice(valueIndex + 1, nextKeyIndex === -1 ? void 0 : nextKeyIndex);
      if (encoded) {
        value = _decodeURI(value);
      }
    }
    if (multiple) {
      if (!(results[name] && Array.isArray(results[name]))) {
        results[name] = [];
      }
      ;
      results[name].push(value);
    } else {
      results[name] ??= value;
    }
  }
  return key ? results[key] : results;
}, "_getQueryParam");
var getQueryParam = _getQueryParam;
var getQueryParams = /* @__PURE__ */ __name((url, key) => {
  return _getQueryParam(url, key, true);
}, "getQueryParams");
var decodeURIComponent_ = decodeURIComponent;

// node_modules/hono/dist/request.js
var tryDecodeURIComponent = /* @__PURE__ */ __name((str) => tryDecode(str, decodeURIComponent_), "tryDecodeURIComponent");
var HonoRequest = /* @__PURE__ */ __name(class {
  /**
   * `.raw` can get the raw Request object.
   *
   * @see {@link https://hono.dev/docs/api/request#raw}
   *
   * @example
   * ```ts
   * // For Cloudflare Workers
   * app.post('/', async (c) => {
   *   const metadata = c.req.raw.cf?.hostMetadata?
   *   ...
   * })
   * ```
   */
  raw;
  #validatedData;
  // Short name of validatedData
  #matchResult;
  routeIndex = 0;
  /**
   * `.path` can get the pathname of the request.
   *
   * @see {@link https://hono.dev/docs/api/request#path}
   *
   * @example
   * ```ts
   * app.get('/about/me', (c) => {
   *   const pathname = c.req.path // `/about/me`
   * })
   * ```
   */
  path;
  bodyCache = {};
  constructor(request, path = "/", matchResult = [[]]) {
    this.raw = request;
    this.path = path;
    this.#matchResult = matchResult;
    this.#validatedData = {};
  }
  param(key) {
    return key ? this.#getDecodedParam(key) : this.#getAllDecodedParams();
  }
  #getDecodedParam(key) {
    const paramKey = this.#matchResult[0][this.routeIndex][1][key];
    const param = this.#getParamValue(paramKey);
    return param && /\%/.test(param) ? tryDecodeURIComponent(param) : param;
  }
  #getAllDecodedParams() {
    const decoded = {};
    const keys = Object.keys(this.#matchResult[0][this.routeIndex][1]);
    for (const key of keys) {
      const value = this.#getParamValue(this.#matchResult[0][this.routeIndex][1][key]);
      if (value !== void 0) {
        decoded[key] = /\%/.test(value) ? tryDecodeURIComponent(value) : value;
      }
    }
    return decoded;
  }
  #getParamValue(paramKey) {
    return this.#matchResult[1] ? this.#matchResult[1][paramKey] : paramKey;
  }
  query(key) {
    return getQueryParam(this.url, key);
  }
  queries(key) {
    return getQueryParams(this.url, key);
  }
  header(name) {
    if (name) {
      return this.raw.headers.get(name) ?? void 0;
    }
    const headerData = {};
    this.raw.headers.forEach((value, key) => {
      headerData[key] = value;
    });
    return headerData;
  }
  async parseBody(options) {
    return this.bodyCache.parsedBody ??= await parseBody(this, options);
  }
  #cachedBody = (key) => {
    const { bodyCache, raw: raw2 } = this;
    const cachedBody = bodyCache[key];
    if (cachedBody) {
      return cachedBody;
    }
    const anyCachedKey = Object.keys(bodyCache)[0];
    if (anyCachedKey) {
      return bodyCache[anyCachedKey].then((body) => {
        if (anyCachedKey === "json") {
          body = JSON.stringify(body);
        }
        return new Response(body)[key]();
      });
    }
    return bodyCache[key] = raw2[key]();
  };
  /**
   * `.json()` can parse Request body of type `application/json`
   *
   * @see {@link https://hono.dev/docs/api/request#json}
   *
   * @example
   * ```ts
   * app.post('/entry', async (c) => {
   *   const body = await c.req.json()
   * })
   * ```
   */
  json() {
    return this.#cachedBody("text").then((text) => JSON.parse(text));
  }
  /**
   * `.text()` can parse Request body of type `text/plain`
   *
   * @see {@link https://hono.dev/docs/api/request#text}
   *
   * @example
   * ```ts
   * app.post('/entry', async (c) => {
   *   const body = await c.req.text()
   * })
   * ```
   */
  text() {
    return this.#cachedBody("text");
  }
  /**
   * `.arrayBuffer()` parse Request body as an `ArrayBuffer`
   *
   * @see {@link https://hono.dev/docs/api/request#arraybuffer}
   *
   * @example
   * ```ts
   * app.post('/entry', async (c) => {
   *   const body = await c.req.arrayBuffer()
   * })
   * ```
   */
  arrayBuffer() {
    return this.#cachedBody("arrayBuffer");
  }
  /**
   * Parses the request body as a `Blob`.
   * @example
   * ```ts
   * app.post('/entry', async (c) => {
   *   const body = await c.req.blob();
   * });
   * ```
   * @see https://hono.dev/docs/api/request#blob
   */
  blob() {
    return this.#cachedBody("blob");
  }
  /**
   * Parses the request body as `FormData`.
   * @example
   * ```ts
   * app.post('/entry', async (c) => {
   *   const body = await c.req.formData();
   * });
   * ```
   * @see https://hono.dev/docs/api/request#formdata
   */
  formData() {
    return this.#cachedBody("formData");
  }
  /**
   * Adds validated data to the request.
   *
   * @param target - The target of the validation.
   * @param data - The validated data to add.
   */
  addValidatedData(target, data) {
    this.#validatedData[target] = data;
  }
  valid(target) {
    return this.#validatedData[target];
  }
  /**
   * `.url()` can get the request url strings.
   *
   * @see {@link https://hono.dev/docs/api/request#url}
   *
   * @example
   * ```ts
   * app.get('/about/me', (c) => {
   *   const url = c.req.url // `http://localhost:8787/about/me`
   *   ...
   * })
   * ```
   */
  get url() {
    return this.raw.url;
  }
  /**
   * `.method()` can get the method name of the request.
   *
   * @see {@link https://hono.dev/docs/api/request#method}
   *
   * @example
   * ```ts
   * app.get('/about/me', (c) => {
   *   const method = c.req.method // `GET`
   * })
   * ```
   */
  get method() {
    return this.raw.method;
  }
  get [GET_MATCH_RESULT]() {
    return this.#matchResult;
  }
  /**
   * `.matchedRoutes()` can return a matched route in the handler
   *
   * @deprecated
   *
   * Use matchedRoutes helper defined in "hono/route" instead.
   *
   * @see {@link https://hono.dev/docs/api/request#matchedroutes}
   *
   * @example
   * ```ts
   * app.use('*', async function logger(c, next) {
   *   await next()
   *   c.req.matchedRoutes.forEach(({ handler, method, path }, i) => {
   *     const name = handler.name || (handler.length < 2 ? '[handler]' : '[middleware]')
   *     console.log(
   *       method,
   *       ' ',
   *       path,
   *       ' '.repeat(Math.max(10 - path.length, 0)),
   *       name,
   *       i === c.req.routeIndex ? '<- respond from here' : ''
   *     )
   *   })
   * })
   * ```
   */
  get matchedRoutes() {
    return this.#matchResult[0].map(([[, route]]) => route);
  }
  /**
   * `routePath()` can retrieve the path registered within the handler
   *
   * @deprecated
   *
   * Use routePath helper defined in "hono/route" instead.
   *
   * @see {@link https://hono.dev/docs/api/request#routepath}
   *
   * @example
   * ```ts
   * app.get('/posts/:id', (c) => {
   *   return c.json({ path: c.req.routePath })
   * })
   * ```
   */
  get routePath() {
    return this.#matchResult[0].map(([[, route]]) => route)[this.routeIndex].path;
  }
}, "HonoRequest");

// node_modules/hono/dist/utils/html.js
var HtmlEscapedCallbackPhase = {
  Stringify: 1,
  BeforeStream: 2,
  Stream: 3
};
var raw = /* @__PURE__ */ __name((value, callbacks) => {
  const escapedString = new String(value);
  escapedString.isEscaped = true;
  escapedString.callbacks = callbacks;
  return escapedString;
}, "raw");
var resolveCallback = /* @__PURE__ */ __name(async (str, phase, preserveCallbacks, context, buffer) => {
  if (typeof str === "object" && !(str instanceof String)) {
    if (!(str instanceof Promise)) {
      str = str.toString();
    }
    if (str instanceof Promise) {
      str = await str;
    }
  }
  const callbacks = str.callbacks;
  if (!callbacks?.length) {
    return Promise.resolve(str);
  }
  if (buffer) {
    buffer[0] += str;
  } else {
    buffer = [str];
  }
  const resStr = Promise.all(callbacks.map((c) => c({ phase, buffer, context }))).then(
    (res) => Promise.all(
      res.filter(Boolean).map((str2) => resolveCallback(str2, phase, false, context, buffer))
    ).then(() => buffer[0])
  );
  if (preserveCallbacks) {
    return raw(await resStr, callbacks);
  } else {
    return resStr;
  }
}, "resolveCallback");

// node_modules/hono/dist/context.js
var TEXT_PLAIN = "text/plain; charset=UTF-8";
var setDefaultContentType = /* @__PURE__ */ __name((contentType, headers) => {
  return {
    "Content-Type": contentType,
    ...headers
  };
}, "setDefaultContentType");
var Context = /* @__PURE__ */ __name(class {
  #rawRequest;
  #req;
  /**
   * `.env` can get bindings (environment variables, secrets, KV namespaces, D1 database, R2 bucket etc.) in Cloudflare Workers.
   *
   * @see {@link https://hono.dev/docs/api/context#env}
   *
   * @example
   * ```ts
   * // Environment object for Cloudflare Workers
   * app.get('*', async c => {
   *   const counter = c.env.COUNTER
   * })
   * ```
   */
  env = {};
  #var;
  finalized = false;
  /**
   * `.error` can get the error object from the middleware if the Handler throws an error.
   *
   * @see {@link https://hono.dev/docs/api/context#error}
   *
   * @example
   * ```ts
   * app.use('*', async (c, next) => {
   *   await next()
   *   if (c.error) {
   *     // do something...
   *   }
   * })
   * ```
   */
  error;
  #status;
  #executionCtx;
  #res;
  #layout;
  #renderer;
  #notFoundHandler;
  #preparedHeaders;
  #matchResult;
  #path;
  /**
   * Creates an instance of the Context class.
   *
   * @param req - The Request object.
   * @param options - Optional configuration options for the context.
   */
  constructor(req, options) {
    this.#rawRequest = req;
    if (options) {
      this.#executionCtx = options.executionCtx;
      this.env = options.env;
      this.#notFoundHandler = options.notFoundHandler;
      this.#path = options.path;
      this.#matchResult = options.matchResult;
    }
  }
  /**
   * `.req` is the instance of {@link HonoRequest}.
   */
  get req() {
    this.#req ??= new HonoRequest(this.#rawRequest, this.#path, this.#matchResult);
    return this.#req;
  }
  /**
   * @see {@link https://hono.dev/docs/api/context#event}
   * The FetchEvent associated with the current request.
   *
   * @throws Will throw an error if the context does not have a FetchEvent.
   */
  get event() {
    if (this.#executionCtx && "respondWith" in this.#executionCtx) {
      return this.#executionCtx;
    } else {
      throw Error("This context has no FetchEvent");
    }
  }
  /**
   * @see {@link https://hono.dev/docs/api/context#executionctx}
   * The ExecutionContext associated with the current request.
   *
   * @throws Will throw an error if the context does not have an ExecutionContext.
   */
  get executionCtx() {
    if (this.#executionCtx) {
      return this.#executionCtx;
    } else {
      throw Error("This context has no ExecutionContext");
    }
  }
  /**
   * @see {@link https://hono.dev/docs/api/context#res}
   * The Response object for the current request.
   */
  get res() {
    return this.#res ||= new Response(null, {
      headers: this.#preparedHeaders ??= new Headers()
    });
  }
  /**
   * Sets the Response object for the current request.
   *
   * @param _res - The Response object to set.
   */
  set res(_res) {
    if (this.#res && _res) {
      _res = new Response(_res.body, _res);
      for (const [k, v] of this.#res.headers.entries()) {
        if (k === "content-type") {
          continue;
        }
        if (k === "set-cookie") {
          const cookies = this.#res.headers.getSetCookie();
          _res.headers.delete("set-cookie");
          for (const cookie of cookies) {
            _res.headers.append("set-cookie", cookie);
          }
        } else {
          _res.headers.set(k, v);
        }
      }
    }
    this.#res = _res;
    this.finalized = true;
  }
  /**
   * `.render()` can create a response within a layout.
   *
   * @see {@link https://hono.dev/docs/api/context#render-setrenderer}
   *
   * @example
   * ```ts
   * app.get('/', (c) => {
   *   return c.render('Hello!')
   * })
   * ```
   */
  render = (...args) => {
    this.#renderer ??= (content) => this.html(content);
    return this.#renderer(...args);
  };
  /**
   * Sets the layout for the response.
   *
   * @param layout - The layout to set.
   * @returns The layout function.
   */
  setLayout = (layout) => this.#layout = layout;
  /**
   * Gets the current layout for the response.
   *
   * @returns The current layout function.
   */
  getLayout = () => this.#layout;
  /**
   * `.setRenderer()` can set the layout in the custom middleware.
   *
   * @see {@link https://hono.dev/docs/api/context#render-setrenderer}
   *
   * @example
   * ```tsx
   * app.use('*', async (c, next) => {
   *   c.setRenderer((content) => {
   *     return c.html(
   *       <html>
   *         <body>
   *           <p>{content}</p>
   *         </body>
   *       </html>
   *     )
   *   })
   *   await next()
   * })
   * ```
   */
  setRenderer = (renderer) => {
    this.#renderer = renderer;
  };
  /**
   * `.header()` can set headers.
   *
   * @see {@link https://hono.dev/docs/api/context#header}
   *
   * @example
   * ```ts
   * app.get('/welcome', (c) => {
   *   // Set headers
   *   c.header('X-Message', 'Hello!')
   *   c.header('Content-Type', 'text/plain')
   *
   *   return c.body('Thank you for coming')
   * })
   * ```
   */
  header = (name, value, options) => {
    if (this.finalized) {
      this.#res = new Response(this.#res.body, this.#res);
    }
    const headers = this.#res ? this.#res.headers : this.#preparedHeaders ??= new Headers();
    if (value === void 0) {
      headers.delete(name);
    } else if (options?.append) {
      headers.append(name, value);
    } else {
      headers.set(name, value);
    }
  };
  status = (status) => {
    this.#status = status;
  };
  /**
   * `.set()` can set the value specified by the key.
   *
   * @see {@link https://hono.dev/docs/api/context#set-get}
   *
   * @example
   * ```ts
   * app.use('*', async (c, next) => {
   *   c.set('message', 'Hono is hot!!')
   *   await next()
   * })
   * ```
   */
  set = (key, value) => {
    this.#var ??= /* @__PURE__ */ new Map();
    this.#var.set(key, value);
  };
  /**
   * `.get()` can use the value specified by the key.
   *
   * @see {@link https://hono.dev/docs/api/context#set-get}
   *
   * @example
   * ```ts
   * app.get('/', (c) => {
   *   const message = c.get('message')
   *   return c.text(`The message is "${message}"`)
   * })
   * ```
   */
  get = (key) => {
    return this.#var ? this.#var.get(key) : void 0;
  };
  /**
   * `.var` can access the value of a variable.
   *
   * @see {@link https://hono.dev/docs/api/context#var}
   *
   * @example
   * ```ts
   * const result = c.var.client.oneMethod()
   * ```
   */
  // c.var.propName is a read-only
  get var() {
    if (!this.#var) {
      return {};
    }
    return Object.fromEntries(this.#var);
  }
  #newResponse(data, arg, headers) {
    const responseHeaders = this.#res ? new Headers(this.#res.headers) : this.#preparedHeaders ?? new Headers();
    if (typeof arg === "object" && "headers" in arg) {
      const argHeaders = arg.headers instanceof Headers ? arg.headers : new Headers(arg.headers);
      for (const [key, value] of argHeaders) {
        if (key.toLowerCase() === "set-cookie") {
          responseHeaders.append(key, value);
        } else {
          responseHeaders.set(key, value);
        }
      }
    }
    if (headers) {
      for (const [k, v] of Object.entries(headers)) {
        if (typeof v === "string") {
          responseHeaders.set(k, v);
        } else {
          responseHeaders.delete(k);
          for (const v2 of v) {
            responseHeaders.append(k, v2);
          }
        }
      }
    }
    const status = typeof arg === "number" ? arg : arg?.status ?? this.#status;
    return new Response(data, { status, headers: responseHeaders });
  }
  newResponse = (...args) => this.#newResponse(...args);
  /**
   * `.body()` can return the HTTP response.
   * You can set headers with `.header()` and set HTTP status code with `.status`.
   * This can also be set in `.text()`, `.json()` and so on.
   *
   * @see {@link https://hono.dev/docs/api/context#body}
   *
   * @example
   * ```ts
   * app.get('/welcome', (c) => {
   *   // Set headers
   *   c.header('X-Message', 'Hello!')
   *   c.header('Content-Type', 'text/plain')
   *   // Set HTTP status code
   *   c.status(201)
   *
   *   // Return the response body
   *   return c.body('Thank you for coming')
   * })
   * ```
   */
  body = (data, arg, headers) => this.#newResponse(data, arg, headers);
  /**
   * `.text()` can render text as `Content-Type:text/plain`.
   *
   * @see {@link https://hono.dev/docs/api/context#text}
   *
   * @example
   * ```ts
   * app.get('/say', (c) => {
   *   return c.text('Hello!')
   * })
   * ```
   */
  text = (text, arg, headers) => {
    return !this.#preparedHeaders && !this.#status && !arg && !headers && !this.finalized ? new Response(text) : this.#newResponse(
      text,
      arg,
      setDefaultContentType(TEXT_PLAIN, headers)
    );
  };
  /**
   * `.json()` can render JSON as `Content-Type:application/json`.
   *
   * @see {@link https://hono.dev/docs/api/context#json}
   *
   * @example
   * ```ts
   * app.get('/api', (c) => {
   *   return c.json({ message: 'Hello!' })
   * })
   * ```
   */
  json = (object, arg, headers) => {
    return this.#newResponse(
      JSON.stringify(object),
      arg,
      setDefaultContentType("application/json", headers)
    );
  };
  html = (html, arg, headers) => {
    const res = /* @__PURE__ */ __name((html2) => this.#newResponse(html2, arg, setDefaultContentType("text/html; charset=UTF-8", headers)), "res");
    return typeof html === "object" ? resolveCallback(html, HtmlEscapedCallbackPhase.Stringify, false, {}).then(res) : res(html);
  };
  /**
   * `.redirect()` can Redirect, default status code is 302.
   *
   * @see {@link https://hono.dev/docs/api/context#redirect}
   *
   * @example
   * ```ts
   * app.get('/redirect', (c) => {
   *   return c.redirect('/')
   * })
   * app.get('/redirect-permanently', (c) => {
   *   return c.redirect('/', 301)
   * })
   * ```
   */
  redirect = (location, status) => {
    const locationString = String(location);
    this.header(
      "Location",
      // Multibyes should be encoded
      // eslint-disable-next-line no-control-regex
      !/[^\x00-\xFF]/.test(locationString) ? locationString : encodeURI(locationString)
    );
    return this.newResponse(null, status ?? 302);
  };
  /**
   * `.notFound()` can return the Not Found Response.
   *
   * @see {@link https://hono.dev/docs/api/context#notfound}
   *
   * @example
   * ```ts
   * app.get('/notfound', (c) => {
   *   return c.notFound()
   * })
   * ```
   */
  notFound = () => {
    this.#notFoundHandler ??= () => new Response();
    return this.#notFoundHandler(this);
  };
}, "Context");

// node_modules/hono/dist/router.js
var METHOD_NAME_ALL = "ALL";
var METHOD_NAME_ALL_LOWERCASE = "all";
var METHODS = ["get", "post", "put", "delete", "options", "patch"];
var MESSAGE_MATCHER_IS_ALREADY_BUILT = "Can not add a route since the matcher is already built.";
var UnsupportedPathError = /* @__PURE__ */ __name(class extends Error {
}, "UnsupportedPathError");

// node_modules/hono/dist/utils/constants.js
var COMPOSED_HANDLER = "__COMPOSED_HANDLER";

// node_modules/hono/dist/hono-base.js
var notFoundHandler = /* @__PURE__ */ __name((c) => {
  return c.text("404 Not Found", 404);
}, "notFoundHandler");
var errorHandler = /* @__PURE__ */ __name((err, c) => {
  if ("getResponse" in err) {
    const res = err.getResponse();
    return c.newResponse(res.body, res);
  }
  console.error(err);
  return c.text("Internal Server Error", 500);
}, "errorHandler");
var Hono = /* @__PURE__ */ __name(class _Hono {
  get;
  post;
  put;
  delete;
  options;
  patch;
  all;
  on;
  use;
  /*
    This class is like an abstract class and does not have a router.
    To use it, inherit the class and implement router in the constructor.
  */
  router;
  getPath;
  // Cannot use `#` because it requires visibility at JavaScript runtime.
  _basePath = "/";
  #path = "/";
  routes = [];
  constructor(options = {}) {
    const allMethods = [...METHODS, METHOD_NAME_ALL_LOWERCASE];
    allMethods.forEach((method) => {
      this[method] = (args1, ...args) => {
        if (typeof args1 === "string") {
          this.#path = args1;
        } else {
          this.#addRoute(method, this.#path, args1);
        }
        args.forEach((handler) => {
          this.#addRoute(method, this.#path, handler);
        });
        return this;
      };
    });
    this.on = (method, path, ...handlers) => {
      for (const p of [path].flat()) {
        this.#path = p;
        for (const m of [method].flat()) {
          handlers.map((handler) => {
            this.#addRoute(m.toUpperCase(), this.#path, handler);
          });
        }
      }
      return this;
    };
    this.use = (arg1, ...handlers) => {
      if (typeof arg1 === "string") {
        this.#path = arg1;
      } else {
        this.#path = "*";
        handlers.unshift(arg1);
      }
      handlers.forEach((handler) => {
        this.#addRoute(METHOD_NAME_ALL, this.#path, handler);
      });
      return this;
    };
    const { strict, ...optionsWithoutStrict } = options;
    Object.assign(this, optionsWithoutStrict);
    this.getPath = strict ?? true ? options.getPath ?? getPath : getPathNoStrict;
  }
  #clone() {
    const clone = new _Hono({
      router: this.router,
      getPath: this.getPath
    });
    clone.errorHandler = this.errorHandler;
    clone.#notFoundHandler = this.#notFoundHandler;
    clone.routes = this.routes;
    return clone;
  }
  #notFoundHandler = notFoundHandler;
  // Cannot use `#` because it requires visibility at JavaScript runtime.
  errorHandler = errorHandler;
  /**
   * `.route()` allows grouping other Hono instance in routes.
   *
   * @see {@link https://hono.dev/docs/api/routing#grouping}
   *
   * @param {string} path - base Path
   * @param {Hono} app - other Hono instance
   * @returns {Hono} routed Hono instance
   *
   * @example
   * ```ts
   * const app = new Hono()
   * const app2 = new Hono()
   *
   * app2.get("/user", (c) => c.text("user"))
   * app.route("/api", app2) // GET /api/user
   * ```
   */
  route(path, app2) {
    const subApp = this.basePath(path);
    app2.routes.map((r) => {
      let handler;
      if (app2.errorHandler === errorHandler) {
        handler = r.handler;
      } else {
        handler = /* @__PURE__ */ __name(async (c, next) => (await compose([], app2.errorHandler)(c, () => r.handler(c, next))).res, "handler");
        handler[COMPOSED_HANDLER] = r.handler;
      }
      subApp.#addRoute(r.method, r.path, handler);
    });
    return this;
  }
  /**
   * `.basePath()` allows base paths to be specified.
   *
   * @see {@link https://hono.dev/docs/api/routing#base-path}
   *
   * @param {string} path - base Path
   * @returns {Hono} changed Hono instance
   *
   * @example
   * ```ts
   * const api = new Hono().basePath('/api')
   * ```
   */
  basePath(path) {
    const subApp = this.#clone();
    subApp._basePath = mergePath(this._basePath, path);
    return subApp;
  }
  /**
   * `.onError()` handles an error and returns a customized Response.
   *
   * @see {@link https://hono.dev/docs/api/hono#error-handling}
   *
   * @param {ErrorHandler} handler - request Handler for error
   * @returns {Hono} changed Hono instance
   *
   * @example
   * ```ts
   * app.onError((err, c) => {
   *   console.error(`${err}`)
   *   return c.text('Custom Error Message', 500)
   * })
   * ```
   */
  onError = (handler) => {
    this.errorHandler = handler;
    return this;
  };
  /**
   * `.notFound()` allows you to customize a Not Found Response.
   *
   * @see {@link https://hono.dev/docs/api/hono#not-found}
   *
   * @param {NotFoundHandler} handler - request handler for not-found
   * @returns {Hono} changed Hono instance
   *
   * @example
   * ```ts
   * app.notFound((c) => {
   *   return c.text('Custom 404 Message', 404)
   * })
   * ```
   */
  notFound = (handler) => {
    this.#notFoundHandler = handler;
    return this;
  };
  /**
   * `.mount()` allows you to mount applications built with other frameworks into your Hono application.
   *
   * @see {@link https://hono.dev/docs/api/hono#mount}
   *
   * @param {string} path - base Path
   * @param {Function} applicationHandler - other Request Handler
   * @param {MountOptions} [options] - options of `.mount()`
   * @returns {Hono} mounted Hono instance
   *
   * @example
   * ```ts
   * import { Router as IttyRouter } from 'itty-router'
   * import { Hono } from 'hono'
   * // Create itty-router application
   * const ittyRouter = IttyRouter()
   * // GET /itty-router/hello
   * ittyRouter.get('/hello', () => new Response('Hello from itty-router'))
   *
   * const app = new Hono()
   * app.mount('/itty-router', ittyRouter.handle)
   * ```
   *
   * @example
   * ```ts
   * const app = new Hono()
   * // Send the request to another application without modification.
   * app.mount('/app', anotherApp, {
   *   replaceRequest: (req) => req,
   * })
   * ```
   */
  mount(path, applicationHandler, options) {
    let replaceRequest;
    let optionHandler;
    if (options) {
      if (typeof options === "function") {
        optionHandler = options;
      } else {
        optionHandler = options.optionHandler;
        if (options.replaceRequest === false) {
          replaceRequest = /* @__PURE__ */ __name((request) => request, "replaceRequest");
        } else {
          replaceRequest = options.replaceRequest;
        }
      }
    }
    const getOptions = optionHandler ? (c) => {
      const options2 = optionHandler(c);
      return Array.isArray(options2) ? options2 : [options2];
    } : (c) => {
      let executionContext = void 0;
      try {
        executionContext = c.executionCtx;
      } catch {
      }
      return [c.env, executionContext];
    };
    replaceRequest ||= (() => {
      const mergedPath = mergePath(this._basePath, path);
      const pathPrefixLength = mergedPath === "/" ? 0 : mergedPath.length;
      return (request) => {
        const url = new URL(request.url);
        url.pathname = url.pathname.slice(pathPrefixLength) || "/";
        return new Request(url, request);
      };
    })();
    const handler = /* @__PURE__ */ __name(async (c, next) => {
      const res = await applicationHandler(replaceRequest(c.req.raw), ...getOptions(c));
      if (res) {
        return res;
      }
      await next();
    }, "handler");
    this.#addRoute(METHOD_NAME_ALL, mergePath(path, "*"), handler);
    return this;
  }
  #addRoute(method, path, handler) {
    method = method.toUpperCase();
    path = mergePath(this._basePath, path);
    const r = { basePath: this._basePath, path, method, handler };
    this.router.add(method, path, [handler, r]);
    this.routes.push(r);
  }
  #handleError(err, c) {
    if (err instanceof Error) {
      return this.errorHandler(err, c);
    }
    throw err;
  }
  #dispatch(request, executionCtx, env, method) {
    if (method === "HEAD") {
      return (async () => new Response(null, await this.#dispatch(request, executionCtx, env, "GET")))();
    }
    const path = this.getPath(request, { env });
    const matchResult = this.router.match(method, path);
    const c = new Context(request, {
      path,
      matchResult,
      env,
      executionCtx,
      notFoundHandler: this.#notFoundHandler
    });
    if (matchResult[0].length === 1) {
      let res;
      try {
        res = matchResult[0][0][0][0](c, async () => {
          c.res = await this.#notFoundHandler(c);
        });
      } catch (err) {
        return this.#handleError(err, c);
      }
      return res instanceof Promise ? res.then(
        (resolved) => resolved || (c.finalized ? c.res : this.#notFoundHandler(c))
      ).catch((err) => this.#handleError(err, c)) : res ?? this.#notFoundHandler(c);
    }
    const composed = compose(matchResult[0], this.errorHandler, this.#notFoundHandler);
    return (async () => {
      try {
        const context = await composed(c);
        if (!context.finalized) {
          throw new Error(
            "Context is not finalized. Did you forget to return a Response object or `await next()`?"
          );
        }
        return context.res;
      } catch (err) {
        return this.#handleError(err, c);
      }
    })();
  }
  /**
   * `.fetch()` will be entry point of your app.
   *
   * @see {@link https://hono.dev/docs/api/hono#fetch}
   *
   * @param {Request} request - request Object of request
   * @param {Env} Env - env Object
   * @param {ExecutionContext} - context of execution
   * @returns {Response | Promise<Response>} response of request
   *
   */
  fetch = (request, ...rest) => {
    return this.#dispatch(request, rest[1], rest[0], request.method);
  };
  /**
   * `.request()` is a useful method for testing.
   * You can pass a URL or pathname to send a GET request.
   * app will return a Response object.
   * ```ts
   * test('GET /hello is ok', async () => {
   *   const res = await app.request('/hello')
   *   expect(res.status).toBe(200)
   * })
   * ```
   * @see https://hono.dev/docs/api/hono#request
   */
  request = (input, requestInit, Env, executionCtx) => {
    if (input instanceof Request) {
      return this.fetch(requestInit ? new Request(input, requestInit) : input, Env, executionCtx);
    }
    input = input.toString();
    return this.fetch(
      new Request(
        /^https?:\/\//.test(input) ? input : `http://localhost${mergePath("/", input)}`,
        requestInit
      ),
      Env,
      executionCtx
    );
  };
  /**
   * `.fire()` automatically adds a global fetch event listener.
   * This can be useful for environments that adhere to the Service Worker API, such as non-ES module Cloudflare Workers.
   * @deprecated
   * Use `fire` from `hono/service-worker` instead.
   * ```ts
   * import { Hono } from 'hono'
   * import { fire } from 'hono/service-worker'
   *
   * const app = new Hono()
   * // ...
   * fire(app)
   * ```
   * @see https://hono.dev/docs/api/hono#fire
   * @see https://developer.mozilla.org/en-US/docs/Web/API/Service_Worker_API
   * @see https://developers.cloudflare.com/workers/reference/migrate-to-module-workers/
   */
  fire = () => {
    addEventListener("fetch", (event) => {
      event.respondWith(this.#dispatch(event.request, event, void 0, event.request.method));
    });
  };
}, "_Hono");

// node_modules/hono/dist/router/reg-exp-router/matcher.js
var emptyParam = [];
function match(method, path) {
  const matchers = this.buildAllMatchers();
  const match2 = /* @__PURE__ */ __name((method2, path2) => {
    const matcher = matchers[method2] || matchers[METHOD_NAME_ALL];
    const staticMatch = matcher[2][path2];
    if (staticMatch) {
      return staticMatch;
    }
    const match3 = path2.match(matcher[0]);
    if (!match3) {
      return [[], emptyParam];
    }
    const index = match3.indexOf("", 1);
    return [matcher[1][index], match3];
  }, "match2");
  this.match = match2;
  return match2(method, path);
}
__name(match, "match");

// node_modules/hono/dist/router/reg-exp-router/node.js
var LABEL_REG_EXP_STR = "[^/]+";
var ONLY_WILDCARD_REG_EXP_STR = ".*";
var TAIL_WILDCARD_REG_EXP_STR = "(?:|/.*)";
var PATH_ERROR = /* @__PURE__ */ Symbol();
var regExpMetaChars = new Set(".\\+*[^]$()");
function compareKey(a, b) {
  if (a.length === 1) {
    return b.length === 1 ? a < b ? -1 : 1 : -1;
  }
  if (b.length === 1) {
    return 1;
  }
  if (a === ONLY_WILDCARD_REG_EXP_STR || a === TAIL_WILDCARD_REG_EXP_STR) {
    return 1;
  } else if (b === ONLY_WILDCARD_REG_EXP_STR || b === TAIL_WILDCARD_REG_EXP_STR) {
    return -1;
  }
  if (a === LABEL_REG_EXP_STR) {
    return 1;
  } else if (b === LABEL_REG_EXP_STR) {
    return -1;
  }
  return a.length === b.length ? a < b ? -1 : 1 : b.length - a.length;
}
__name(compareKey, "compareKey");
var Node = /* @__PURE__ */ __name(class _Node {
  #index;
  #varIndex;
  #children = /* @__PURE__ */ Object.create(null);
  insert(tokens, index, paramMap, context, pathErrorCheckOnly) {
    if (tokens.length === 0) {
      if (this.#index !== void 0) {
        throw PATH_ERROR;
      }
      if (pathErrorCheckOnly) {
        return;
      }
      this.#index = index;
      return;
    }
    const [token, ...restTokens] = tokens;
    const pattern = token === "*" ? restTokens.length === 0 ? ["", "", ONLY_WILDCARD_REG_EXP_STR] : ["", "", LABEL_REG_EXP_STR] : token === "/*" ? ["", "", TAIL_WILDCARD_REG_EXP_STR] : token.match(/^\:([^\{\}]+)(?:\{(.+)\})?$/);
    let node;
    if (pattern) {
      const name = pattern[1];
      let regexpStr = pattern[2] || LABEL_REG_EXP_STR;
      if (name && pattern[2]) {
        if (regexpStr === ".*") {
          throw PATH_ERROR;
        }
        regexpStr = regexpStr.replace(/^\((?!\?:)(?=[^)]+\)$)/, "(?:");
        if (/\((?!\?:)/.test(regexpStr)) {
          throw PATH_ERROR;
        }
      }
      node = this.#children[regexpStr];
      if (!node) {
        if (Object.keys(this.#children).some(
          (k) => k !== ONLY_WILDCARD_REG_EXP_STR && k !== TAIL_WILDCARD_REG_EXP_STR
        )) {
          throw PATH_ERROR;
        }
        if (pathErrorCheckOnly) {
          return;
        }
        node = this.#children[regexpStr] = new _Node();
        if (name !== "") {
          node.#varIndex = context.varIndex++;
        }
      }
      if (!pathErrorCheckOnly && name !== "") {
        paramMap.push([name, node.#varIndex]);
      }
    } else {
      node = this.#children[token];
      if (!node) {
        if (Object.keys(this.#children).some(
          (k) => k.length > 1 && k !== ONLY_WILDCARD_REG_EXP_STR && k !== TAIL_WILDCARD_REG_EXP_STR
        )) {
          throw PATH_ERROR;
        }
        if (pathErrorCheckOnly) {
          return;
        }
        node = this.#children[token] = new _Node();
      }
    }
    node.insert(restTokens, index, paramMap, context, pathErrorCheckOnly);
  }
  buildRegExpStr() {
    const childKeys = Object.keys(this.#children).sort(compareKey);
    const strList = childKeys.map((k) => {
      const c = this.#children[k];
      return (typeof c.#varIndex === "number" ? `(${k})@${c.#varIndex}` : regExpMetaChars.has(k) ? `\\${k}` : k) + c.buildRegExpStr();
    });
    if (typeof this.#index === "number") {
      strList.unshift(`#${this.#index}`);
    }
    if (strList.length === 0) {
      return "";
    }
    if (strList.length === 1) {
      return strList[0];
    }
    return "(?:" + strList.join("|") + ")";
  }
}, "_Node");

// node_modules/hono/dist/router/reg-exp-router/trie.js
var Trie = /* @__PURE__ */ __name(class {
  #context = { varIndex: 0 };
  #root = new Node();
  insert(path, index, pathErrorCheckOnly) {
    const paramAssoc = [];
    const groups = [];
    for (let i = 0; ; ) {
      let replaced = false;
      path = path.replace(/\{[^}]+\}/g, (m) => {
        const mark = `@\\${i}`;
        groups[i] = [mark, m];
        i++;
        replaced = true;
        return mark;
      });
      if (!replaced) {
        break;
      }
    }
    const tokens = path.match(/(?::[^\/]+)|(?:\/\*$)|./g) || [];
    for (let i = groups.length - 1; i >= 0; i--) {
      const [mark] = groups[i];
      for (let j = tokens.length - 1; j >= 0; j--) {
        if (tokens[j].indexOf(mark) !== -1) {
          tokens[j] = tokens[j].replace(mark, groups[i][1]);
          break;
        }
      }
    }
    this.#root.insert(tokens, index, paramAssoc, this.#context, pathErrorCheckOnly);
    return paramAssoc;
  }
  buildRegExp() {
    let regexp = this.#root.buildRegExpStr();
    if (regexp === "") {
      return [/^$/, [], []];
    }
    let captureIndex = 0;
    const indexReplacementMap = [];
    const paramReplacementMap = [];
    regexp = regexp.replace(/#(\d+)|@(\d+)|\.\*\$/g, (_, handlerIndex, paramIndex) => {
      if (handlerIndex !== void 0) {
        indexReplacementMap[++captureIndex] = Number(handlerIndex);
        return "$()";
      }
      if (paramIndex !== void 0) {
        paramReplacementMap[Number(paramIndex)] = ++captureIndex;
        return "";
      }
      return "";
    });
    return [new RegExp(`^${regexp}`), indexReplacementMap, paramReplacementMap];
  }
}, "Trie");

// node_modules/hono/dist/router/reg-exp-router/router.js
var nullMatcher = [/^$/, [], /* @__PURE__ */ Object.create(null)];
var wildcardRegExpCache = /* @__PURE__ */ Object.create(null);
function buildWildcardRegExp(path) {
  return wildcardRegExpCache[path] ??= new RegExp(
    path === "*" ? "" : `^${path.replace(
      /\/\*$|([.\\+*[^\]$()])/g,
      (_, metaChar) => metaChar ? `\\${metaChar}` : "(?:|/.*)"
    )}$`
  );
}
__name(buildWildcardRegExp, "buildWildcardRegExp");
function clearWildcardRegExpCache() {
  wildcardRegExpCache = /* @__PURE__ */ Object.create(null);
}
__name(clearWildcardRegExpCache, "clearWildcardRegExpCache");
function buildMatcherFromPreprocessedRoutes(routes) {
  const trie = new Trie();
  const handlerData = [];
  if (routes.length === 0) {
    return nullMatcher;
  }
  const routesWithStaticPathFlag = routes.map(
    (route) => [!/\*|\/:/.test(route[0]), ...route]
  ).sort(
    ([isStaticA, pathA], [isStaticB, pathB]) => isStaticA ? 1 : isStaticB ? -1 : pathA.length - pathB.length
  );
  const staticMap = /* @__PURE__ */ Object.create(null);
  for (let i = 0, j = -1, len = routesWithStaticPathFlag.length; i < len; i++) {
    const [pathErrorCheckOnly, path, handlers] = routesWithStaticPathFlag[i];
    if (pathErrorCheckOnly) {
      staticMap[path] = [handlers.map(([h]) => [h, /* @__PURE__ */ Object.create(null)]), emptyParam];
    } else {
      j++;
    }
    let paramAssoc;
    try {
      paramAssoc = trie.insert(path, j, pathErrorCheckOnly);
    } catch (e) {
      throw e === PATH_ERROR ? new UnsupportedPathError(path) : e;
    }
    if (pathErrorCheckOnly) {
      continue;
    }
    handlerData[j] = handlers.map(([h, paramCount]) => {
      const paramIndexMap = /* @__PURE__ */ Object.create(null);
      paramCount -= 1;
      for (; paramCount >= 0; paramCount--) {
        const [key, value] = paramAssoc[paramCount];
        paramIndexMap[key] = value;
      }
      return [h, paramIndexMap];
    });
  }
  const [regexp, indexReplacementMap, paramReplacementMap] = trie.buildRegExp();
  for (let i = 0, len = handlerData.length; i < len; i++) {
    for (let j = 0, len2 = handlerData[i].length; j < len2; j++) {
      const map = handlerData[i][j]?.[1];
      if (!map) {
        continue;
      }
      const keys = Object.keys(map);
      for (let k = 0, len3 = keys.length; k < len3; k++) {
        map[keys[k]] = paramReplacementMap[map[keys[k]]];
      }
    }
  }
  const handlerMap = [];
  for (const i in indexReplacementMap) {
    handlerMap[i] = handlerData[indexReplacementMap[i]];
  }
  return [regexp, handlerMap, staticMap];
}
__name(buildMatcherFromPreprocessedRoutes, "buildMatcherFromPreprocessedRoutes");
function findMiddleware(middleware, path) {
  if (!middleware) {
    return void 0;
  }
  for (const k of Object.keys(middleware).sort((a, b) => b.length - a.length)) {
    if (buildWildcardRegExp(k).test(path)) {
      return [...middleware[k]];
    }
  }
  return void 0;
}
__name(findMiddleware, "findMiddleware");
var RegExpRouter = /* @__PURE__ */ __name(class {
  name = "RegExpRouter";
  #middleware;
  #routes;
  constructor() {
    this.#middleware = { [METHOD_NAME_ALL]: /* @__PURE__ */ Object.create(null) };
    this.#routes = { [METHOD_NAME_ALL]: /* @__PURE__ */ Object.create(null) };
  }
  add(method, path, handler) {
    const middleware = this.#middleware;
    const routes = this.#routes;
    if (!middleware || !routes) {
      throw new Error(MESSAGE_MATCHER_IS_ALREADY_BUILT);
    }
    if (!middleware[method]) {
      ;
      [middleware, routes].forEach((handlerMap) => {
        handlerMap[method] = /* @__PURE__ */ Object.create(null);
        Object.keys(handlerMap[METHOD_NAME_ALL]).forEach((p) => {
          handlerMap[method][p] = [...handlerMap[METHOD_NAME_ALL][p]];
        });
      });
    }
    if (path === "/*") {
      path = "*";
    }
    const paramCount = (path.match(/\/:/g) || []).length;
    if (/\*$/.test(path)) {
      const re = buildWildcardRegExp(path);
      if (method === METHOD_NAME_ALL) {
        Object.keys(middleware).forEach((m) => {
          middleware[m][path] ||= findMiddleware(middleware[m], path) || findMiddleware(middleware[METHOD_NAME_ALL], path) || [];
        });
      } else {
        middleware[method][path] ||= findMiddleware(middleware[method], path) || findMiddleware(middleware[METHOD_NAME_ALL], path) || [];
      }
      Object.keys(middleware).forEach((m) => {
        if (method === METHOD_NAME_ALL || method === m) {
          Object.keys(middleware[m]).forEach((p) => {
            re.test(p) && middleware[m][p].push([handler, paramCount]);
          });
        }
      });
      Object.keys(routes).forEach((m) => {
        if (method === METHOD_NAME_ALL || method === m) {
          Object.keys(routes[m]).forEach(
            (p) => re.test(p) && routes[m][p].push([handler, paramCount])
          );
        }
      });
      return;
    }
    const paths = checkOptionalParameter(path) || [path];
    for (let i = 0, len = paths.length; i < len; i++) {
      const path2 = paths[i];
      Object.keys(routes).forEach((m) => {
        if (method === METHOD_NAME_ALL || method === m) {
          routes[m][path2] ||= [
            ...findMiddleware(middleware[m], path2) || findMiddleware(middleware[METHOD_NAME_ALL], path2) || []
          ];
          routes[m][path2].push([handler, paramCount - len + i + 1]);
        }
      });
    }
  }
  match = match;
  buildAllMatchers() {
    const matchers = /* @__PURE__ */ Object.create(null);
    Object.keys(this.#routes).concat(Object.keys(this.#middleware)).forEach((method) => {
      matchers[method] ||= this.#buildMatcher(method);
    });
    this.#middleware = this.#routes = void 0;
    clearWildcardRegExpCache();
    return matchers;
  }
  #buildMatcher(method) {
    const routes = [];
    let hasOwnRoute = method === METHOD_NAME_ALL;
    [this.#middleware, this.#routes].forEach((r) => {
      const ownRoute = r[method] ? Object.keys(r[method]).map((path) => [path, r[method][path]]) : [];
      if (ownRoute.length !== 0) {
        hasOwnRoute ||= true;
        routes.push(...ownRoute);
      } else if (method !== METHOD_NAME_ALL) {
        routes.push(
          ...Object.keys(r[METHOD_NAME_ALL]).map((path) => [path, r[METHOD_NAME_ALL][path]])
        );
      }
    });
    if (!hasOwnRoute) {
      return null;
    } else {
      return buildMatcherFromPreprocessedRoutes(routes);
    }
  }
}, "RegExpRouter");

// node_modules/hono/dist/router/smart-router/router.js
var SmartRouter = /* @__PURE__ */ __name(class {
  name = "SmartRouter";
  #routers = [];
  #routes = [];
  constructor(init) {
    this.#routers = init.routers;
  }
  add(method, path, handler) {
    if (!this.#routes) {
      throw new Error(MESSAGE_MATCHER_IS_ALREADY_BUILT);
    }
    this.#routes.push([method, path, handler]);
  }
  match(method, path) {
    if (!this.#routes) {
      throw new Error("Fatal error");
    }
    const routers = this.#routers;
    const routes = this.#routes;
    const len = routers.length;
    let i = 0;
    let res;
    for (; i < len; i++) {
      const router = routers[i];
      try {
        for (let i2 = 0, len2 = routes.length; i2 < len2; i2++) {
          router.add(...routes[i2]);
        }
        res = router.match(method, path);
      } catch (e) {
        if (e instanceof UnsupportedPathError) {
          continue;
        }
        throw e;
      }
      this.match = router.match.bind(router);
      this.#routers = [router];
      this.#routes = void 0;
      break;
    }
    if (i === len) {
      throw new Error("Fatal error");
    }
    this.name = `SmartRouter + ${this.activeRouter.name}`;
    return res;
  }
  get activeRouter() {
    if (this.#routes || this.#routers.length !== 1) {
      throw new Error("No active router has been determined yet.");
    }
    return this.#routers[0];
  }
}, "SmartRouter");

// node_modules/hono/dist/router/trie-router/node.js
var emptyParams = /* @__PURE__ */ Object.create(null);
var Node2 = /* @__PURE__ */ __name(class _Node2 {
  #methods;
  #children;
  #patterns;
  #order = 0;
  #params = emptyParams;
  constructor(method, handler, children) {
    this.#children = children || /* @__PURE__ */ Object.create(null);
    this.#methods = [];
    if (method && handler) {
      const m = /* @__PURE__ */ Object.create(null);
      m[method] = { handler, possibleKeys: [], score: 0 };
      this.#methods = [m];
    }
    this.#patterns = [];
  }
  insert(method, path, handler) {
    this.#order = ++this.#order;
    let curNode = this;
    const parts = splitRoutingPath(path);
    const possibleKeys = [];
    for (let i = 0, len = parts.length; i < len; i++) {
      const p = parts[i];
      const nextP = parts[i + 1];
      const pattern = getPattern(p, nextP);
      const key = Array.isArray(pattern) ? pattern[0] : p;
      if (key in curNode.#children) {
        curNode = curNode.#children[key];
        if (pattern) {
          possibleKeys.push(pattern[1]);
        }
        continue;
      }
      curNode.#children[key] = new _Node2();
      if (pattern) {
        curNode.#patterns.push(pattern);
        possibleKeys.push(pattern[1]);
      }
      curNode = curNode.#children[key];
    }
    curNode.#methods.push({
      [method]: {
        handler,
        possibleKeys: possibleKeys.filter((v, i, a) => a.indexOf(v) === i),
        score: this.#order
      }
    });
    return curNode;
  }
  #getHandlerSets(node, method, nodeParams, params) {
    const handlerSets = [];
    for (let i = 0, len = node.#methods.length; i < len; i++) {
      const m = node.#methods[i];
      const handlerSet = m[method] || m[METHOD_NAME_ALL];
      const processedSet = {};
      if (handlerSet !== void 0) {
        handlerSet.params = /* @__PURE__ */ Object.create(null);
        handlerSets.push(handlerSet);
        if (nodeParams !== emptyParams || params && params !== emptyParams) {
          for (let i2 = 0, len2 = handlerSet.possibleKeys.length; i2 < len2; i2++) {
            const key = handlerSet.possibleKeys[i2];
            const processed = processedSet[handlerSet.score];
            handlerSet.params[key] = params?.[key] && !processed ? params[key] : nodeParams[key] ?? params?.[key];
            processedSet[handlerSet.score] = true;
          }
        }
      }
    }
    return handlerSets;
  }
  search(method, path) {
    const handlerSets = [];
    this.#params = emptyParams;
    const curNode = this;
    let curNodes = [curNode];
    const parts = splitPath(path);
    const curNodesQueue = [];
    for (let i = 0, len = parts.length; i < len; i++) {
      const part = parts[i];
      const isLast = i === len - 1;
      const tempNodes = [];
      for (let j = 0, len2 = curNodes.length; j < len2; j++) {
        const node = curNodes[j];
        const nextNode = node.#children[part];
        if (nextNode) {
          nextNode.#params = node.#params;
          if (isLast) {
            if (nextNode.#children["*"]) {
              handlerSets.push(
                ...this.#getHandlerSets(nextNode.#children["*"], method, node.#params)
              );
            }
            handlerSets.push(...this.#getHandlerSets(nextNode, method, node.#params));
          } else {
            tempNodes.push(nextNode);
          }
        }
        for (let k = 0, len3 = node.#patterns.length; k < len3; k++) {
          const pattern = node.#patterns[k];
          const params = node.#params === emptyParams ? {} : { ...node.#params };
          if (pattern === "*") {
            const astNode = node.#children["*"];
            if (astNode) {
              handlerSets.push(...this.#getHandlerSets(astNode, method, node.#params));
              astNode.#params = params;
              tempNodes.push(astNode);
            }
            continue;
          }
          const [key, name, matcher] = pattern;
          if (!part && !(matcher instanceof RegExp)) {
            continue;
          }
          const child = node.#children[key];
          const restPathString = parts.slice(i).join("/");
          if (matcher instanceof RegExp) {
            const m = matcher.exec(restPathString);
            if (m) {
              params[name] = m[0];
              handlerSets.push(...this.#getHandlerSets(child, method, node.#params, params));
              if (Object.keys(child.#children).length) {
                child.#params = params;
                const componentCount = m[0].match(/\//)?.length ?? 0;
                const targetCurNodes = curNodesQueue[componentCount] ||= [];
                targetCurNodes.push(child);
              }
              continue;
            }
          }
          if (matcher === true || matcher.test(part)) {
            params[name] = part;
            if (isLast) {
              handlerSets.push(...this.#getHandlerSets(child, method, params, node.#params));
              if (child.#children["*"]) {
                handlerSets.push(
                  ...this.#getHandlerSets(child.#children["*"], method, params, node.#params)
                );
              }
            } else {
              child.#params = params;
              tempNodes.push(child);
            }
          }
        }
      }
      curNodes = tempNodes.concat(curNodesQueue.shift() ?? []);
    }
    if (handlerSets.length > 1) {
      handlerSets.sort((a, b) => {
        return a.score - b.score;
      });
    }
    return [handlerSets.map(({ handler, params }) => [handler, params])];
  }
}, "_Node");

// node_modules/hono/dist/router/trie-router/router.js
var TrieRouter = /* @__PURE__ */ __name(class {
  name = "TrieRouter";
  #node;
  constructor() {
    this.#node = new Node2();
  }
  add(method, path, handler) {
    const results = checkOptionalParameter(path);
    if (results) {
      for (let i = 0, len = results.length; i < len; i++) {
        this.#node.insert(method, results[i], handler);
      }
      return;
    }
    this.#node.insert(method, path, handler);
  }
  match(method, path) {
    return this.#node.search(method, path);
  }
}, "TrieRouter");

// node_modules/hono/dist/hono.js
var Hono2 = /* @__PURE__ */ __name(class extends Hono {
  /**
   * Creates an instance of the Hono class.
   *
   * @param options - Optional configuration options for the Hono instance.
   */
  constructor(options = {}) {
    super(options);
    this.router = options.router ?? new SmartRouter({
      routers: [new RegExpRouter(), new TrieRouter()]
    });
  }
}, "Hono");

// node_modules/hono/dist/middleware/cors/index.js
var cors = /* @__PURE__ */ __name((options) => {
  const defaults = {
    origin: "*",
    allowMethods: ["GET", "HEAD", "PUT", "POST", "DELETE", "PATCH"],
    allowHeaders: [],
    exposeHeaders: []
  };
  const opts = {
    ...defaults,
    ...options
  };
  const findAllowOrigin = ((optsOrigin) => {
    if (typeof optsOrigin === "string") {
      if (optsOrigin === "*") {
        return () => optsOrigin;
      } else {
        return (origin) => optsOrigin === origin ? origin : null;
      }
    } else if (typeof optsOrigin === "function") {
      return optsOrigin;
    } else {
      return (origin) => optsOrigin.includes(origin) ? origin : null;
    }
  })(opts.origin);
  const findAllowMethods = ((optsAllowMethods) => {
    if (typeof optsAllowMethods === "function") {
      return optsAllowMethods;
    } else if (Array.isArray(optsAllowMethods)) {
      return () => optsAllowMethods;
    } else {
      return () => [];
    }
  })(opts.allowMethods);
  return /* @__PURE__ */ __name(async function cors2(c, next) {
    function set(key, value) {
      c.res.headers.set(key, value);
    }
    __name(set, "set");
    const allowOrigin = await findAllowOrigin(c.req.header("origin") || "", c);
    if (allowOrigin) {
      set("Access-Control-Allow-Origin", allowOrigin);
    }
    if (opts.credentials) {
      set("Access-Control-Allow-Credentials", "true");
    }
    if (opts.exposeHeaders?.length) {
      set("Access-Control-Expose-Headers", opts.exposeHeaders.join(","));
    }
    if (c.req.method === "OPTIONS") {
      if (opts.origin !== "*") {
        set("Vary", "Origin");
      }
      if (opts.maxAge != null) {
        set("Access-Control-Max-Age", opts.maxAge.toString());
      }
      const allowMethods = await findAllowMethods(c.req.header("origin") || "", c);
      if (allowMethods.length) {
        set("Access-Control-Allow-Methods", allowMethods.join(","));
      }
      let headers = opts.allowHeaders;
      if (!headers?.length) {
        const requestHeaders = c.req.header("Access-Control-Request-Headers");
        if (requestHeaders) {
          headers = requestHeaders.split(/\s*,\s*/);
        }
      }
      if (headers?.length) {
        set("Access-Control-Allow-Headers", headers.join(","));
        c.res.headers.append("Vary", "Access-Control-Request-Headers");
      }
      c.res.headers.delete("Content-Length");
      c.res.headers.delete("Content-Type");
      return new Response(null, {
        headers: c.res.headers,
        status: 204,
        statusText: "No Content"
      });
    }
    await next();
    if (opts.origin !== "*") {
      c.header("Vary", "Origin", { append: true });
    }
  }, "cors2");
}, "cors");

// node_modules/hono/dist/utils/color.js
function getColorEnabled() {
  const { process, Deno } = globalThis;
  const isNoColor = typeof Deno?.noColor === "boolean" ? Deno.noColor : process !== void 0 ? (
    // eslint-disable-next-line no-unsafe-optional-chaining
    "NO_COLOR" in process?.env
  ) : false;
  return !isNoColor;
}
__name(getColorEnabled, "getColorEnabled");
async function getColorEnabledAsync() {
  const { navigator } = globalThis;
  const cfWorkers = "cloudflare:workers";
  const isNoColor = navigator !== void 0 && navigator.userAgent === "Cloudflare-Workers" ? await (async () => {
    try {
      return "NO_COLOR" in ((await import(cfWorkers)).env ?? {});
    } catch {
      return false;
    }
  })() : !getColorEnabled();
  return !isNoColor;
}
__name(getColorEnabledAsync, "getColorEnabledAsync");

// node_modules/hono/dist/middleware/logger/index.js
var humanize = /* @__PURE__ */ __name((times) => {
  const [delimiter, separator] = [",", "."];
  const orderTimes = times.map((v) => v.replace(/(\d)(?=(\d\d\d)+(?!\d))/g, "$1" + delimiter));
  return orderTimes.join(separator);
}, "humanize");
var time = /* @__PURE__ */ __name((start) => {
  const delta = Date.now() - start;
  return humanize([delta < 1e3 ? delta + "ms" : Math.round(delta / 1e3) + "s"]);
}, "time");
var colorStatus = /* @__PURE__ */ __name(async (status) => {
  const colorEnabled = await getColorEnabledAsync();
  if (colorEnabled) {
    switch (status / 100 | 0) {
      case 5:
        return `\x1B[31m${status}\x1B[0m`;
      case 4:
        return `\x1B[33m${status}\x1B[0m`;
      case 3:
        return `\x1B[36m${status}\x1B[0m`;
      case 2:
        return `\x1B[32m${status}\x1B[0m`;
    }
  }
  return `${status}`;
}, "colorStatus");
async function log(fn, prefix, method, path, status = 0, elapsed) {
  const out = prefix === "<--" ? `${prefix} ${method} ${path}` : `${prefix} ${method} ${path} ${await colorStatus(status)} ${elapsed}`;
  fn(out);
}
__name(log, "log");
var logger = /* @__PURE__ */ __name((fn = console.log) => {
  return /* @__PURE__ */ __name(async function logger2(c, next) {
    const { method, url } = c.req;
    const path = url.slice(url.indexOf("/", 8));
    await log(fn, "<--", method, path);
    const start = Date.now();
    await next();
    await log(fn, "-->", method, path, c.res.status, time(start));
  }, "logger2");
}, "logger");

// node_modules/hono/dist/middleware/pretty-json/index.js
var prettyJSON = /* @__PURE__ */ __name((options) => {
  const targetQuery = options?.query ?? "pretty";
  return /* @__PURE__ */ __name(async function prettyJSON2(c, next) {
    const pretty = options?.force || c.req.query(targetQuery) || c.req.query(targetQuery) === "";
    await next();
    if (pretty && c.res.headers.get("Content-Type")?.startsWith("application/json")) {
      const obj = await c.res.json();
      c.res = new Response(JSON.stringify(obj, null, options?.space ?? 2), c.res);
    }
  }, "prettyJSON2");
}, "prettyJSON");

// src/core/TextUtils.ts
var DEFAULT_OPTIONS = {
  lowercase: true,
  removePunctuation: true,
  removeNumbers: false,
  minTokenLength: 1
};
function tokenize(text, options) {
  const opts = { ...DEFAULT_OPTIONS, ...options };
  if (!text || typeof text !== "string") {
    return [];
  }
  let processed = text;
  if (opts.lowercase) {
    processed = processed.toLowerCase();
  }
  if (opts.removePunctuation) {
    processed = processed.replace(/[^\p{L}\p{N}\s]/gu, " ");
  }
  let tokens = processed.split(/\s+/).filter((t) => t.length > 0);
  if (opts.removeNumbers) {
    tokens = tokens.filter((t) => !/^\d+$/.test(t));
  }
  if (opts.minTokenLength && opts.minTokenLength > 1) {
    const minLen = opts.minTokenLength;
    tokens = tokens.filter((t) => t.length >= minLen);
  }
  return tokens;
}
__name(tokenize, "tokenize");
function getTermFrequencies(tokens) {
  const frequencies = /* @__PURE__ */ new Map();
  for (const token of tokens) {
    const current = frequencies.get(token) || 0;
    frequencies.set(token, current + 1);
  }
  return frequencies;
}
__name(getTermFrequencies, "getTermFrequencies");
function getNestedValue(obj, path) {
  const parts = path.split(".");
  let current = obj;
  for (const part of parts) {
    if (current === null || current === void 0)
      return void 0;
    if (typeof current !== "object")
      return void 0;
    current = current[part];
  }
  return current;
}
__name(getNestedValue, "getNestedValue");
function extractTextFromMetadata(metadata, textFields) {
  if (!metadata) {
    return "";
  }
  const textParts = [];
  for (const field of textFields) {
    const value = getNestedValue(metadata, field);
    if (typeof value === "string") {
      textParts.push(value);
    } else if (Array.isArray(value)) {
      const stringValues = value.filter((v) => typeof v === "string");
      textParts.push(...stringValues);
    }
  }
  return textParts.join(" ");
}
__name(extractTextFromMetadata, "extractTextFromMetadata");

// src/core/BM25Index.ts
var BM25Index = class {
  k1;
  b;
  textFields;
  tokenizerOptions;
  // Document storage: id -> document data
  documents = /* @__PURE__ */ new Map();
  // Inverted index: term -> count of documents containing this term
  documentFrequencies = /* @__PURE__ */ new Map();
  // Statistics
  totalDocLength = 0;
  constructor(options) {
    this.k1 = options.k1 ?? 1.2;
    this.b = options.b ?? 0.75;
    this.textFields = options.textFields;
    this.tokenizerOptions = options.tokenizerOptions ?? {};
  }
  /**
   * Gets the text fields being indexed
   */
  get indexedFields() {
    return [...this.textFields];
  }
  /**
   * Gets the number of documents in the index
   */
  get documentCount() {
    return this.documents.size;
  }
  /**
   * Gets the average document length
   */
  get avgDocLength() {
    if (this.documents.size === 0)
      return 0;
    return this.totalDocLength / this.documents.size;
  }
  /**
   * Gets the vocabulary size (unique terms)
   */
  get vocabularySize() {
    return this.documentFrequencies.size;
  }
  /**
   * Adds a document to the index
   */
  addDocument(id, metadata) {
    if (this.documents.has(id)) {
      this.removeDocument(id);
    }
    const text = extractTextFromMetadata(metadata, this.textFields);
    const tokens = tokenize(text, this.tokenizerOptions);
    const termFrequencies = getTermFrequencies(tokens);
    const docLength = tokens.length;
    const seenTerms = /* @__PURE__ */ new Set();
    for (const term of tokens) {
      if (!seenTerms.has(term)) {
        seenTerms.add(term);
        const current = this.documentFrequencies.get(term) || 0;
        this.documentFrequencies.set(term, current + 1);
      }
    }
    this.documents.set(id, {
      length: docLength,
      termFrequencies,
      metadata: metadata || void 0
    });
    this.totalDocLength += docLength;
  }
  /**
   * Updates a document in the index
   */
  updateDocument(id, metadata) {
    this.addDocument(id, metadata);
  }
  /**
   * Removes a document from the index
   */
  removeDocument(id) {
    const doc = this.documents.get(id);
    if (!doc)
      return false;
    for (const [term] of doc.termFrequencies) {
      const current = this.documentFrequencies.get(term) || 0;
      if (current <= 1) {
        this.documentFrequencies.delete(term);
      } else {
        this.documentFrequencies.set(term, current - 1);
      }
    }
    this.totalDocLength -= doc.length;
    this.documents.delete(id);
    return true;
  }
  /**
   * Calculates the IDF (Inverse Document Frequency) for a term
   */
  calculateIDF(term) {
    const n = this.documentFrequencies.get(term) || 0;
    const N = this.documents.size;
    if (n === 0)
      return 0;
    return Math.log((N - n + 0.5) / (n + 0.5) + 1);
  }
  /**
   * Calculates the BM25 score for a document given query terms
   */
  calculateScore(docId, queryTerms) {
    const doc = this.documents.get(docId);
    if (!doc)
      return 0;
    const avgdl = this.avgDocLength;
    if (avgdl === 0)
      return 0;
    let score = 0;
    for (const term of queryTerms) {
      const idf = this.calculateIDF(term);
      const tf = doc.termFrequencies.get(term) || 0;
      if (tf === 0)
        continue;
      const numerator = tf * (this.k1 + 1);
      const denominator = tf + this.k1 * (1 - this.b + this.b * (doc.length / avgdl));
      score += idf * (numerator / denominator);
    }
    return score;
  }
  /**
   * Searches the index and returns k most relevant documents
   */
  search(query, k) {
    if (this.documents.size === 0 || !query.trim()) {
      return [];
    }
    const queryTerms = tokenize(query, this.tokenizerOptions);
    if (queryTerms.length === 0) {
      return [];
    }
    const scores = [];
    for (const [docId] of this.documents) {
      const score = this.calculateScore(docId, queryTerms);
      if (score > 0) {
        scores.push({ id: docId, score });
      }
    }
    scores.sort((a, b) => b.score - a.score);
    return scores.slice(0, k).map(({ id, score }) => {
      const doc = this.documents.get(id);
      return {
        id,
        score,
        metadata: doc?.metadata
      };
    });
  }
  /**
   * Gets statistics about the index
   */
  getStats() {
    return {
      documentCount: this.documentCount,
      avgDocLength: this.avgDocLength,
      vocabularySize: this.vocabularySize,
      k1: this.k1,
      b: this.b,
      textFields: this.textFields
    };
  }
  /**
   * Serializes the index to a plain object
   */
  serialize() {
    const documents = [];
    for (const [id, doc] of this.documents) {
      const termFrequencies = {};
      for (const [term, freq] of doc.termFrequencies) {
        termFrequencies[term] = freq;
      }
      documents.push({
        id,
        length: doc.length,
        termFrequencies
      });
    }
    const documentFrequencies = {};
    for (const [term, freq] of this.documentFrequencies) {
      documentFrequencies[term] = freq;
    }
    return {
      version: "1.0.0",
      k1: this.k1,
      b: this.b,
      textFields: this.textFields,
      avgDocLength: this.avgDocLength,
      documentCount: this.documentCount,
      documents,
      documentFrequencies
    };
  }
  /**
   * Deserializes an index from a plain object
   */
  static deserialize(data) {
    const index = new BM25Index({
      k1: data.k1,
      b: data.b,
      textFields: data.textFields
    });
    for (const doc of data.documents) {
      const termFrequencies = /* @__PURE__ */ new Map();
      for (const [term, freq] of Object.entries(doc.termFrequencies)) {
        termFrequencies.set(term, freq);
      }
      index.documents.set(doc.id, {
        length: doc.length,
        termFrequencies
      });
      index.totalDocLength += doc.length;
    }
    for (const [term, freq] of Object.entries(data.documentFrequencies)) {
      index.documentFrequencies.set(term, freq);
    }
    return index;
  }
  /**
   * Clears the entire index
   */
  clear() {
    this.documents.clear();
    this.documentFrequencies.clear();
    this.totalDocLength = 0;
  }
};
__name(BM25Index, "BM25Index");

// src/core/HybridSearch.ts
function reciprocalRankFusion(vectorResults, keywordResults, k, rrfConstant = 60) {
  const vectorRanks = /* @__PURE__ */ new Map();
  const keywordRanks = /* @__PURE__ */ new Map();
  vectorResults.forEach((result, index) => {
    vectorRanks.set(result.id, index + 1);
  });
  keywordResults.forEach((result, index) => {
    keywordRanks.set(result.id, index + 1);
  });
  const allIds = /* @__PURE__ */ new Set([
    ...vectorRanks.keys(),
    ...keywordRanks.keys()
  ]);
  const scores = [];
  for (const id of allIds) {
    const vectorRank = vectorRanks.get(id);
    const keywordRank = keywordRanks.get(id);
    let score = 0;
    if (vectorRank !== void 0) {
      score += 1 / (rrfConstant + vectorRank);
    }
    if (keywordRank !== void 0) {
      score += 1 / (rrfConstant + keywordRank);
    }
    const vectorResult = vectorResults.find((r) => r.id === id);
    const keywordResult = keywordResults.find((r) => r.id === id);
    scores.push({
      id,
      score,
      vectorRank,
      keywordRank,
      vectorSimilarity: vectorResult?.similarity,
      keywordScore: keywordResult?.score,
      metadata: vectorResult?.metadata || keywordResult?.metadata
    });
  }
  scores.sort((a, b) => b.score - a.score);
  return scores.slice(0, k);
}
__name(reciprocalRankFusion, "reciprocalRankFusion");
function normalizeScores(results) {
  if (results.length === 0) {
    return /* @__PURE__ */ new Map();
  }
  const scores = results.map((r) => r.score);
  const minScore = Math.min(...scores);
  const maxScore = Math.max(...scores);
  const range = maxScore - minScore;
  const normalized = /* @__PURE__ */ new Map();
  for (const result of results) {
    const normScore = range === 0 ? 1 : (result.score - minScore) / range;
    normalized.set(result.id, normScore);
  }
  return normalized;
}
__name(normalizeScores, "normalizeScores");
function weightedCombination(vectorResults, keywordResults, k, alpha = 0.5) {
  alpha = Math.max(0, Math.min(1, alpha));
  const normalizedVector = /* @__PURE__ */ new Map();
  for (const result of vectorResults) {
    normalizedVector.set(result.id, result.similarity);
  }
  const normalizedKeyword = normalizeScores(
    keywordResults.map((r) => ({ id: r.id, score: r.score }))
  );
  const allIds = /* @__PURE__ */ new Set([
    ...normalizedVector.keys(),
    ...normalizedKeyword.keys()
  ]);
  const scores = [];
  for (const id of allIds) {
    const vectorScore = normalizedVector.get(id) ?? 0;
    const keywordScore = normalizedKeyword.get(id) ?? 0;
    const combinedScore = alpha * vectorScore + (1 - alpha) * keywordScore;
    const vectorResult = vectorResults.find((r) => r.id === id);
    const keywordResult = keywordResults.find((r) => r.id === id);
    const vectorRank = vectorResults.findIndex((r) => r.id === id);
    const keywordRank = keywordResults.findIndex((r) => r.id === id);
    scores.push({
      id,
      score: combinedScore,
      vectorRank: vectorRank >= 0 ? vectorRank + 1 : void 0,
      keywordRank: keywordRank >= 0 ? keywordRank + 1 : void 0,
      vectorSimilarity: vectorResult?.similarity,
      keywordScore: keywordResult?.score,
      metadata: vectorResult?.metadata || keywordResult?.metadata
    });
  }
  scores.sort((a, b) => b.score - a.score);
  return scores.slice(0, k);
}
__name(weightedCombination, "weightedCombination");
function hybridFusion(vectorResults, keywordResults, k, method = "rrf", options) {
  if (method === "rrf") {
    return reciprocalRankFusion(
      vectorResults,
      keywordResults,
      k,
      options?.rrfConstant ?? 60
    );
  } else {
    return weightedCombination(
      vectorResults,
      keywordResults,
      k,
      options?.alpha ?? 0.5
    );
  }
}
__name(hybridFusion, "hybridFusion");

// src/core/Quantization.ts
function quantizeScalar(vector) {
  const quantized = new Int8Array(vector.length);
  for (let i = 0; i < vector.length; i++) {
    const clamped = Math.max(-1, Math.min(1, vector[i]));
    quantized[i] = Math.round(clamped * 127);
  }
  return quantized;
}
__name(quantizeScalar, "quantizeScalar");
function cosineSimilarityInt8(a, b) {
  let dot = 0;
  let normA = 0;
  let normB = 0;
  const len = Math.min(a.length, b.length);
  for (let i = 0; i < len; i++) {
    dot += a[i] * b[i];
    normA += a[i] * a[i];
    normB += b[i] * b[i];
  }
  const denom = Math.sqrt(normA) * Math.sqrt(normB);
  if (denom === 0)
    return 0;
  return dot / denom;
}
__name(cosineSimilarityInt8, "cosineSimilarityInt8");
function quantizeBinary(vector) {
  const bytes = Math.ceil(vector.length / 8);
  const quantized = new Uint8Array(bytes);
  for (let i = 0; i < vector.length; i++) {
    if (vector[i] > 0) {
      quantized[Math.floor(i / 8)] |= 1 << i % 8;
    }
  }
  return quantized;
}
__name(quantizeBinary, "quantizeBinary");
function hammingDistance(a, b) {
  let distance = 0;
  const len = Math.min(a.length, b.length);
  for (let i = 0; i < len; i++) {
    let xor = a[i] ^ b[i];
    while (xor) {
      distance += xor & 1;
      xor >>>= 1;
    }
  }
  return distance;
}
__name(hammingDistance, "hammingDistance");
function hammingToSimilarity(distance, totalBits) {
  return 1 - distance / totalBits;
}
__name(hammingToSimilarity, "hammingToSimilarity");
function int8ToBase64(arr) {
  const bytes = new Uint8Array(arr.buffer, arr.byteOffset, arr.byteLength);
  let binary = "";
  for (let i = 0; i < bytes.length; i++) {
    binary += String.fromCharCode(bytes[i]);
  }
  return btoa(binary);
}
__name(int8ToBase64, "int8ToBase64");
function base64ToInt8(base64) {
  const binary = atob(base64);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i++) {
    bytes[i] = binary.charCodeAt(i);
  }
  return new Int8Array(bytes.buffer);
}
__name(base64ToInt8, "base64ToInt8");
function uint8ToBase64(arr) {
  let binary = "";
  for (let i = 0; i < arr.length; i++) {
    binary += String.fromCharCode(arr[i]);
  }
  return btoa(binary);
}
__name(uint8ToBase64, "uint8ToBase64");
function base64ToUint8(base64) {
  const binary = atob(base64);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i++) {
    bytes[i] = binary.charCodeAt(i);
  }
  return bytes;
}
__name(base64ToUint8, "base64ToUint8");
function getQuantizedSize(dims, type) {
  switch (type) {
    case "none":
      return dims * 4;
    case "int8":
      return dims;
    case "binary":
      return Math.ceil(dims / 8);
    default:
      return dims * 4;
  }
}
__name(getQuantizedSize, "getQuantizedSize");
function calculateSavings(dims, type) {
  const original = dims * 4;
  const quantized = getQuantizedSize(dims, type);
  return (original - quantized) / original * 100;
}
__name(calculateSavings, "calculateSavings");

// src/core/VectorDB.ts
function cosineDistance(a, b, normA, normB) {
  let dot = 0;
  let nA = normA ?? 0;
  let nB = normB ?? 0;
  const needNormA = normA === void 0;
  const needNormB = normB === void 0;
  for (let i = 0; i < a.length; i++) {
    dot += a[i] * b[i];
    if (needNormA)
      nA += a[i] * a[i];
    if (needNormB)
      nB += b[i] * b[i];
  }
  if (needNormA)
    nA = Math.sqrt(nA);
  if (needNormB)
    nB = Math.sqrt(nB);
  const denom = nA * nB;
  if (denom === 0)
    return 1;
  const similarity = Math.max(-1, Math.min(1, dot / denom));
  return 1 - similarity;
}
__name(cosineDistance, "cosineDistance");
function euclideanDistance(a, b) {
  let sum = 0;
  for (let i = 0; i < a.length; i++) {
    const diff = a[i] - b[i];
    sum += diff * diff;
  }
  return Math.sqrt(sum);
}
__name(euclideanDistance, "euclideanDistance");
function dotProductDistance(a, b) {
  let dot = 0;
  for (let i = 0; i < a.length; i++) {
    dot += a[i] * b[i];
  }
  return -dot;
}
__name(dotProductDistance, "dotProductDistance");
function getNestedValue2(obj, path) {
  if (!obj)
    return void 0;
  const parts = path.split(".");
  let current = obj;
  for (const part of parts) {
    if (current === null || current === void 0)
      return void 0;
    if (typeof current !== "object")
      return void 0;
    current = current[part];
  }
  return current;
}
__name(getNestedValue2, "getNestedValue");
function compareValues(a, b) {
  if (a instanceof Date && b instanceof Date) {
    return a.getTime() - b.getTime();
  }
  if (typeof a === "string" && typeof b === "string") {
    const dateA = Date.parse(a);
    const dateB = Date.parse(b);
    if (!isNaN(dateA) && !isNaN(dateB)) {
      return dateA - dateB;
    }
    return a.localeCompare(b);
  }
  if (typeof a === "number" && typeof b === "number") {
    return a - b;
  }
  return String(a).localeCompare(String(b));
}
__name(compareValues, "compareValues");
function isFilterCondition(value) {
  if (!value || typeof value !== "object")
    return false;
  const keys = Object.keys(value);
  return keys.length > 0 && keys.every((k) => k.startsWith("$"));
}
__name(isFilterCondition, "isFilterCondition");
function evaluateCondition(fieldValue, condition) {
  const entries = Object.entries(condition);
  for (const [operator, operand] of entries) {
    switch (operator) {
      case "$eq":
        if (fieldValue !== operand)
          return false;
        break;
      case "$ne":
        if (fieldValue === operand)
          return false;
        break;
      case "$gt":
        if (fieldValue === void 0 || compareValues(fieldValue, operand) <= 0)
          return false;
        break;
      case "$gte":
        if (fieldValue === void 0 || compareValues(fieldValue, operand) < 0)
          return false;
        break;
      case "$lt":
        if (fieldValue === void 0 || compareValues(fieldValue, operand) >= 0)
          return false;
        break;
      case "$lte":
        if (fieldValue === void 0 || compareValues(fieldValue, operand) > 0)
          return false;
        break;
      case "$in":
        if (!Array.isArray(operand) || !operand.includes(fieldValue))
          return false;
        break;
      case "$nin":
        if (!Array.isArray(operand) || operand.includes(fieldValue))
          return false;
        break;
      case "$exists":
        if (operand === true && fieldValue === void 0)
          return false;
        if (operand === false && fieldValue !== void 0)
          return false;
        break;
      case "$contains":
        if (typeof fieldValue !== "string" || typeof operand !== "string")
          return false;
        if (!fieldValue.toLowerCase().includes(operand.toLowerCase()))
          return false;
        break;
      case "$startsWith":
        if (typeof fieldValue !== "string" || typeof operand !== "string")
          return false;
        if (!fieldValue.toLowerCase().startsWith(operand.toLowerCase()))
          return false;
        break;
      case "$endsWith":
        if (typeof fieldValue !== "string" || typeof operand !== "string")
          return false;
        if (!fieldValue.toLowerCase().endsWith(operand.toLowerCase()))
          return false;
        break;
    }
  }
  return true;
}
__name(evaluateCondition, "evaluateCondition");
function evaluateFilter(metadata, filter) {
  if (filter.$and) {
    for (const subFilter of filter.$and) {
      if (!evaluateFilter(metadata, subFilter)) {
        return false;
      }
    }
  }
  if (filter.$or) {
    let anyMatch = false;
    for (const subFilter of filter.$or) {
      if (evaluateFilter(metadata, subFilter)) {
        anyMatch = true;
        break;
      }
    }
    if (!anyMatch && filter.$or.length > 0) {
      return false;
    }
  }
  for (const [field, condition] of Object.entries(filter)) {
    if (field === "$and" || field === "$or")
      continue;
    const fieldValue = getNestedValue2(metadata, field);
    if (isFilterCondition(condition)) {
      if (!evaluateCondition(fieldValue, condition)) {
        return false;
      }
    } else {
      if (fieldValue !== condition) {
        return false;
      }
    }
  }
  return true;
}
__name(evaluateFilter, "evaluateFilter");
var VectorDB = class {
  vectors = /* @__PURE__ */ new Map();
  _dimensions;
  _distance;
  _indexType;
  _quantization;
  _rescoreOversample;
  bm25Index = null;
  bm25TextFields = [];
  constructor(options) {
    this._dimensions = options.dimensions;
    this._distance = options.distance || "cosine";
    this._indexType = options.indexType || "flat";
    this._quantization = options.quantization || "none";
    this._rescoreOversample = options.rescoreOversample || 4;
  }
  get dimensions() {
    return this._dimensions;
  }
  get distance() {
    return this._distance;
  }
  get indexType() {
    return this._indexType;
  }
  get quantization() {
    return this._quantization;
  }
  get length() {
    return this.vectors.size;
  }
  computeNorm(vector) {
    let sum = 0;
    for (let i = 0; i < vector.length; i++) {
      sum += vector[i] * vector[i];
    }
    return Math.sqrt(sum);
  }
  calculateDistance(a, b, normA, normB) {
    switch (this._distance) {
      case "cosine":
        return cosineDistance(a, b, normA, normB);
      case "euclidean":
        return euclideanDistance(a, b);
      case "dot":
        return dotProductDistance(a, b);
      default:
        return cosineDistance(a, b, normA, normB);
    }
  }
  /**
   * Create quantized representations for a vector
   */
  createQuantizedRepresentations(vector) {
    const result = {};
    if (this._quantization === "int8" || this._quantization === "binary") {
      result.quantizedInt8 = quantizeScalar(vector);
    }
    if (this._quantization === "binary") {
      result.quantizedBinary = quantizeBinary(vector);
    }
    return result;
  }
  insert(id, vector, metadata) {
    if (vector.length !== this._dimensions) {
      throw new Error(`Dimension mismatch: expected ${this._dimensions}, got ${vector.length}`);
    }
    if (this.vectors.has(id)) {
      throw new Error(`Vector with id "${id}" already exists. Use upsert() instead.`);
    }
    const now = Date.now();
    const norm = this._distance === "cosine" ? this.computeNorm(vector) : void 0;
    const quantized = this.createQuantizedRepresentations(vector);
    this.vectors.set(id, {
      id,
      vector: [...vector],
      metadata: metadata || null,
      norm,
      createdAt: now,
      updatedAt: now,
      ...quantized
    });
    if (this.bm25Index) {
      this.bm25Index.addDocument(id, metadata || null);
    }
  }
  upsert(id, vector, metadata) {
    if (vector.length !== this._dimensions) {
      throw new Error(`Dimension mismatch: expected ${this._dimensions}, got ${vector.length}`);
    }
    const existing = this.vectors.get(id);
    const now = Date.now();
    const norm = this._distance === "cosine" ? this.computeNorm(vector) : void 0;
    const quantized = this.createQuantizedRepresentations(vector);
    this.vectors.set(id, {
      id,
      vector: [...vector],
      metadata: metadata || null,
      norm,
      createdAt: existing?.createdAt ?? now,
      updatedAt: now,
      ...quantized
    });
    if (this.bm25Index) {
      this.bm25Index.updateDocument(id, metadata || null);
    }
  }
  /**
   * Search using quantized vectors (fast approximate search)
   */
  searchQuantized(query, k, filter) {
    if (this._quantization === "binary") {
      const queryBinary = quantizeBinary(query);
      const results = [];
      for (const stored of this.vectors.values()) {
        if (filter && !evaluateFilter(stored.metadata, filter)) {
          continue;
        }
        if (stored.quantizedBinary) {
          const distance = hammingDistance(queryBinary, stored.quantizedBinary);
          const similarity = hammingToSimilarity(distance, this._dimensions);
          results.push({
            id: stored.id,
            similarity,
            metadata: stored.metadata || void 0
          });
        }
      }
      results.sort((a, b) => b.similarity - a.similarity);
      return results.slice(0, k);
    } else if (this._quantization === "int8") {
      const queryInt8 = quantizeScalar(query);
      const results = [];
      for (const stored of this.vectors.values()) {
        if (filter && !evaluateFilter(stored.metadata, filter)) {
          continue;
        }
        if (stored.quantizedInt8) {
          const similarity = cosineSimilarityInt8(queryInt8, stored.quantizedInt8);
          results.push({
            id: stored.id,
            similarity,
            metadata: stored.metadata || void 0
          });
        }
      }
      results.sort((a, b) => b.similarity - a.similarity);
      return results.slice(0, k);
    }
    return [];
  }
  /**
   * Rescore candidates using full-precision vectors
   */
  rescoreCandidates(query, candidates) {
    const queryNorm = this._distance === "cosine" ? this.computeNorm(query) : void 0;
    return candidates.map((candidate) => {
      const stored = this.vectors.get(candidate.id);
      if (!stored) {
        return {
          id: candidate.id,
          distance: 1,
          similarity: candidate.similarity,
          metadata: candidate.metadata
        };
      }
      const distance = this.calculateDistance(query, stored.vector, queryNorm, stored.norm);
      let similarity;
      if (this._distance === "cosine") {
        similarity = 1 - distance;
      } else if (this._distance === "dot") {
        similarity = -distance;
      } else {
        similarity = 1 / (1 + distance);
      }
      return {
        id: stored.id,
        distance,
        similarity,
        metadata: stored.metadata || void 0
      };
    }).sort((a, b) => a.distance - b.distance);
  }
  search(query, k, options) {
    if (query.length !== this._dimensions) {
      throw new Error(`Query dimension mismatch: expected ${this._dimensions}, got ${query.length}`);
    }
    if (this.vectors.size === 0) {
      return [];
    }
    const filter = options?.filter;
    const minSimilarity = options?.minSimilarity ?? 0;
    if (this._quantization !== "none") {
      const fetchK = this._quantization === "binary" ? k * this._rescoreOversample : k;
      const candidates = this.searchQuantized(query, fetchK, filter);
      let results2;
      if (this._quantization === "binary") {
        results2 = this.rescoreCandidates(query, candidates).slice(0, k);
      } else {
        results2 = candidates.map((c) => ({
          id: c.id,
          distance: 1 - c.similarity,
          similarity: c.similarity,
          metadata: c.metadata
        }));
      }
      return results2.filter((r) => r.similarity >= minSimilarity);
    }
    const queryNorm = this._distance === "cosine" ? this.computeNorm(query) : void 0;
    const results = [];
    for (const stored of this.vectors.values()) {
      if (filter && !evaluateFilter(stored.metadata, filter)) {
        continue;
      }
      const distance = this.calculateDistance(query, stored.vector, queryNorm, stored.norm);
      let similarity;
      if (this._distance === "cosine") {
        similarity = 1 - distance;
      } else if (this._distance === "dot") {
        similarity = -distance;
      } else {
        similarity = 1 / (1 + distance);
      }
      if (similarity < minSimilarity) {
        continue;
      }
      results.push({
        id: stored.id,
        distance,
        similarity,
        metadata: stored.metadata || void 0
      });
    }
    results.sort((a, b) => a.distance - b.distance);
    return results.slice(0, Math.min(k, results.length));
  }
  get(id) {
    const stored = this.vectors.get(id);
    if (!stored)
      return null;
    return {
      id: stored.id,
      vector: [...stored.vector],
      metadata: stored.metadata,
      createdAt: stored.createdAt,
      updatedAt: stored.updatedAt
    };
  }
  delete(id) {
    const deleted = this.vectors.delete(id);
    if (deleted && this.bm25Index) {
      this.bm25Index.removeDocument(id);
    }
    return deleted;
  }
  contains(id) {
    return this.vectors.has(id);
  }
  clear() {
    this.vectors.clear();
    if (this.bm25Index) {
      this.bm25Index.clear();
    }
  }
  getIds() {
    return Array.from(this.vectors.keys());
  }
  // ============================================================================
  // BM25 Keyword Search
  // ============================================================================
  configureBM25(options) {
    if (this.bm25Index && JSON.stringify(this.bm25TextFields) === JSON.stringify(options.textFields)) {
      return;
    }
    this.bm25Index = new BM25Index({
      textFields: options.textFields,
      k1: options.k1,
      b: options.b
    });
    this.bm25TextFields = [...options.textFields];
    for (const stored of this.vectors.values()) {
      this.bm25Index.addDocument(stored.id, stored.metadata);
    }
  }
  ensureBM25Index(textFields, k1, b) {
    if (!this.bm25Index || JSON.stringify(this.bm25TextFields) !== JSON.stringify(textFields)) {
      this.configureBM25({ textFields, k1, b });
    }
  }
  keywordSearch(query, k, options) {
    const textFields = options?.textFields || this.bm25TextFields;
    if (textFields.length === 0) {
      throw new Error("No text fields specified for keyword search.");
    }
    this.ensureBM25Index(textFields, options?.k1, options?.b);
    let results = this.bm25Index.search(query, k * 2);
    if (options?.filter) {
      results = results.filter((r) => {
        const stored = this.vectors.get(r.id);
        return stored && evaluateFilter(stored.metadata, options.filter);
      });
    }
    return results.slice(0, k);
  }
  hybridSearch(options) {
    const {
      mode,
      k,
      queryVector,
      keywords,
      textFields = this.bm25TextFields.length > 0 ? this.bm25TextFields : ["content", "text", "title"],
      filter,
      minSimilarity,
      alpha = 0.5,
      fusionMethod = "rrf",
      rrfConstant = 60,
      bm25K1,
      bm25B
    } = options;
    if (mode === "vector") {
      if (!queryVector)
        throw new Error("queryVector is required for vector search mode");
      return this.search(queryVector, k, { filter, minSimilarity }).map((r) => ({
        id: r.id,
        score: r.similarity,
        vectorSimilarity: r.similarity,
        metadata: r.metadata
      }));
    }
    if (mode === "keyword") {
      if (!keywords)
        throw new Error("keywords is required for keyword search mode");
      return this.keywordSearch(keywords, k, { textFields, filter, k1: bm25K1, b: bm25B }).map((r) => ({
        id: r.id,
        score: r.score,
        keywordScore: r.score,
        metadata: r.metadata
      }));
    }
    if (!queryVector)
      throw new Error("queryVector is required for hybrid search mode");
    if (!keywords)
      throw new Error("keywords is required for hybrid search mode");
    const fetchK = Math.max(k * 3, 50);
    const vectorResults = this.search(queryVector, fetchK, { filter, minSimilarity });
    const keywordResults = this.keywordSearch(keywords, fetchK, { textFields, filter, k1: bm25K1, b: bm25B });
    const vectorForFusion = vectorResults.map((r) => ({
      id: r.id,
      distance: r.distance,
      similarity: r.similarity,
      metadata: r.metadata
    }));
    return hybridFusion(vectorForFusion, keywordResults, k, fusionMethod, { alpha, rrfConstant });
  }
  // ============================================================================
  // Serialization
  // ============================================================================
  export() {
    const serializedVectors = [];
    for (const stored of this.vectors.values()) {
      const serialized = {
        id: stored.id,
        vector: stored.vector,
        metadata: stored.metadata,
        norm: stored.norm,
        createdAt: stored.createdAt,
        updatedAt: stored.updatedAt
      };
      if (stored.quantizedInt8) {
        serialized.quantizedInt8 = int8ToBase64(stored.quantizedInt8);
      }
      if (stored.quantizedBinary) {
        serialized.quantizedBinary = uint8ToBase64(stored.quantizedBinary);
      }
      serializedVectors.push(serialized);
    }
    const data = {
      version: "3.0.0",
      // Version bump for quantization support
      dimensions: this._dimensions,
      distance: this._distance,
      indexType: this._indexType,
      quantization: this._quantization,
      vectors: serializedVectors
    };
    if (this.bm25Index) {
      data.bm25Index = this.bm25Index.serialize();
    }
    return data;
  }
  static import(data) {
    const db = new VectorDB({
      dimensions: data.dimensions,
      distance: data.distance,
      indexType: data.indexType,
      quantization: data.quantization
    });
    for (const stored of data.vectors) {
      const entry = {
        id: stored.id,
        vector: stored.vector,
        metadata: stored.metadata,
        norm: stored.norm,
        createdAt: stored.createdAt ?? Date.now(),
        updatedAt: stored.updatedAt ?? Date.now()
      };
      if (stored.quantizedInt8) {
        entry.quantizedInt8 = base64ToInt8(stored.quantizedInt8);
      }
      if (stored.quantizedBinary) {
        entry.quantizedBinary = base64ToUint8(stored.quantizedBinary);
      }
      db.vectors.set(stored.id, entry);
    }
    if (data.bm25Index) {
      db.bm25Index = BM25Index.deserialize(data.bm25Index);
      db.bm25TextFields = data.bm25Index.textFields;
    }
    return db;
  }
  stats() {
    const vectorCount = this.vectors.size;
    const float32Size = vectorCount * this._dimensions * 4;
    const quantizedSize = vectorCount * getQuantizedSize(this._dimensions, this._quantization);
    const stats = {
      dimensions: this._dimensions,
      distance: this._distance,
      indexType: this._indexType,
      quantization: this._quantization,
      vectorCount,
      memoryEstimate: {
        float32MB: float32Size / (1024 * 1024),
        quantizedMB: quantizedSize / (1024 * 1024),
        savingsPercent: calculateSavings(this._dimensions, this._quantization)
      }
    };
    if (this.bm25Index) {
      stats.bm25 = this.bm25Index.getStats();
    }
    return stats;
  }
};
__name(VectorDB, "VectorDB");

// src/memory/MemoryManager.ts
function generateId() {
  return `mem_${Date.now().toString(36)}_${Math.random().toString(36).substring(2, 9)}`;
}
__name(generateId, "generateId");
var MemoryManager = class {
  db;
  dimensions;
  textFields;
  decayRate;
  workingMemoryTTL;
  constructor(options) {
    this.dimensions = options.dimensions;
    this.textFields = options.textFields || ["content", "event", "fact", "context"];
    this.decayRate = options.decayRate ?? 0.01;
    this.workingMemoryTTL = options.workingMemoryTTL ?? 36e5;
    this.db = new VectorDB({
      dimensions: options.dimensions,
      distance: "cosine",
      indexType: "flat"
    });
    this.db.configureBM25({ textFields: this.textFields });
  }
  /**
   * Remember - Store a new memory
   */
  async remember(content, embedding, options = {}) {
    const now = Date.now();
    const id = generateId();
    const type = options.type || "episodic";
    const importance = options.importance ?? 0.5;
    const baseMemory = {
      id,
      type,
      content,
      embedding,
      metadata: options.metadata || {},
      importance,
      createdAt: now,
      updatedAt: now,
      accessedAt: now,
      accessCount: 0
    };
    const fullMetadata = {
      ...baseMemory.metadata,
      type,
      content,
      importance,
      createdAt: now,
      accessCount: 0
    };
    if (type === "working" && options.sessionId) {
      const ttl = options.ttl || this.workingMemoryTTL;
      fullMetadata.sessionId = options.sessionId;
      fullMetadata.ttl = ttl;
      fullMetadata.expiresAt = now + ttl;
    }
    this.db.insert(id, embedding, fullMetadata);
    return baseMemory;
  }
  /**
   * Recall - Search for relevant memories
   */
  async recall(query, options = {}) {
    const limit = options.limit || 10;
    const mode = options.mode || "hybrid";
    const searchOptions = {
      mode,
      k: limit,
      minSimilarity: options.minSimilarity,
      alpha: options.alpha ?? 0.7
      // Favor vector similarity by default
    };
    if (typeof query === "string") {
      searchOptions.keywords = query;
      if (mode === "keyword") {
      } else {
        throw new Error('Hybrid/vector search requires an embedding. Use recallWithEmbedding() or provide mode: "keyword"');
      }
    } else {
      searchOptions.queryVector = query;
      if (mode === "hybrid") {
        throw new Error("Hybrid search requires keywords. Use recallWithEmbedding() with keywords option");
      }
    }
    const filter = {};
    if (options.type) {
      filter.type = options.type;
    }
    if (options.minImportance !== void 0) {
      filter.importance = { $gte: options.minImportance };
    }
    if (options.sessionId) {
      filter.sessionId = options.sessionId;
    }
    filter.$or = [
      { type: { $ne: "working" } },
      { expiresAt: { $gt: Date.now() } }
    ];
    if (Object.keys(filter).length > 1) {
      searchOptions.filter = filter;
    }
    const results = this.db.hybridSearch(searchOptions);
    const now = Date.now();
    for (const result of results) {
      const stored = this.db.get(result.id);
      if (stored?.metadata) {
        const newMetadata = {
          ...stored.metadata,
          accessedAt: now,
          accessCount: (stored.metadata.accessCount || 0) + 1
        };
        this.db.upsert(result.id, stored.vector, newMetadata);
      }
    }
    return results.map((r) => ({
      memory: this.resultToMemory(r.id, r.metadata || null),
      score: r.score,
      vectorSimilarity: r.vectorSimilarity,
      keywordScore: r.keywordScore
    }));
  }
  /**
   * Recall with embedding for hybrid search
   */
  async recallWithEmbedding(keywords, embedding, options = {}) {
    const limit = options.limit || 10;
    const mode = options.mode || "hybrid";
    const searchOptions = {
      mode,
      k: limit,
      queryVector: embedding,
      keywords,
      minSimilarity: options.minSimilarity,
      alpha: options.alpha ?? 0.7
    };
    const filter = {};
    if (options.type) {
      filter.type = options.type;
    }
    if (options.minImportance !== void 0) {
      filter.importance = { $gte: options.minImportance };
    }
    if (options.sessionId) {
      filter.sessionId = options.sessionId;
    }
    if (Object.keys(filter).length > 0) {
      searchOptions.filter = filter;
    }
    const results = this.db.hybridSearch(searchOptions);
    return results.map((r) => ({
      memory: this.resultToMemory(r.id, r.metadata || null),
      score: r.score,
      vectorSimilarity: r.vectorSimilarity,
      keywordScore: r.keywordScore
    }));
  }
  /**
   * Forget - Delete a memory
   */
  async forget(id) {
    return this.db.delete(id);
  }
  /**
   * Forget by filter - Delete multiple memories
   */
  async forgetByFilter(filter) {
    const ids = this.db.getIds();
    let count = 0;
    for (const id of ids) {
      const stored = this.db.get(id);
      if (stored?.metadata) {
        let matches = true;
        for (const [key, value] of Object.entries(filter)) {
          if (stored.metadata[key] !== value) {
            matches = false;
            break;
          }
        }
        if (matches) {
          this.db.delete(id);
          count++;
        }
      }
    }
    return count;
  }
  /**
   * Get a specific memory by ID
   */
  async get(id) {
    const stored = this.db.get(id);
    if (!stored)
      return null;
    return this.resultToMemory(id, stored.metadata);
  }
  /**
   * Update a memory
   */
  async update(id, updates, newEmbedding) {
    const stored = this.db.get(id);
    if (!stored)
      return null;
    const now = Date.now();
    const newMetadata = {
      ...stored.metadata,
      ...updates.metadata,
      updatedAt: now
    };
    if (updates.content !== void 0) {
      newMetadata.content = updates.content;
    }
    if (updates.importance !== void 0) {
      newMetadata.importance = updates.importance;
    }
    const embedding = newEmbedding || stored.vector;
    this.db.upsert(id, embedding, newMetadata);
    return this.resultToMemory(id, newMetadata);
  }
  /**
   * Apply decay to all memories
   */
  async applyDecay() {
    const ids = this.db.getIds();
    const now = Date.now();
    const dayInMs = 864e5;
    for (const id of ids) {
      const stored = this.db.get(id);
      if (!stored?.metadata)
        continue;
      const createdAt = stored.metadata.createdAt || now;
      const daysOld = (now - createdAt) / dayInMs;
      const currentImportance = stored.metadata.importance || 0.5;
      const newImportance = Math.max(0, currentImportance * Math.pow(1 - this.decayRate, daysOld));
      if (newImportance !== currentImportance) {
        this.db.upsert(id, stored.vector, {
          ...stored.metadata,
          importance: newImportance
        });
      }
    }
  }
  /**
   * Clean up expired working memories
   */
  async cleanupExpired() {
    const ids = this.db.getIds();
    const now = Date.now();
    let count = 0;
    for (const id of ids) {
      const stored = this.db.get(id);
      if (!stored?.metadata)
        continue;
      if (stored.metadata.type === "working") {
        const expiresAt = stored.metadata.expiresAt;
        if (expiresAt && expiresAt < now) {
          this.db.delete(id);
          count++;
        }
      }
    }
    return count;
  }
  /**
   * Get memory statistics
   */
  async stats() {
    const ids = this.db.getIds();
    let episodic = 0;
    let semantic = 0;
    let working = 0;
    let totalImportance = 0;
    let oldest = Infinity;
    let newest = 0;
    let knowledge = 0;
    for (const id of ids) {
      const stored = this.db.get(id);
      if (!stored?.metadata)
        continue;
      const type = stored.metadata.type;
      if (type === "episodic")
        episodic++;
      else if (type === "semantic")
        semantic++;
      else if (type === "working")
        working++;
      else if (type === "knowledge")
        knowledge++;
      totalImportance += stored.metadata.importance || 0;
      const createdAt = stored.metadata.createdAt || Date.now();
      if (createdAt < oldest)
        oldest = createdAt;
      if (createdAt > newest)
        newest = createdAt;
    }
    return {
      total: ids.length,
      byType: { episodic, semantic, working, knowledge },
      averageImportance: ids.length > 0 ? totalImportance / ids.length : 0,
      oldestMemory: oldest !== Infinity ? oldest : void 0,
      newestMemory: newest > 0 ? newest : void 0
    };
  }
  /**
   * Export all memories
   */
  export() {
    const ids = this.db.getIds();
    const memories = [];
    for (const id of ids) {
      const stored = this.db.get(id);
      if (stored?.metadata) {
        memories.push({
          ...this.resultToMemory(id, stored.metadata),
          embedding: stored.vector
        });
      }
    }
    return { version: "1.0.0", memories };
  }
  /**
   * Import memories
   */
  import(data) {
    let count = 0;
    for (const memory of data.memories) {
      if (memory.embedding) {
        this.db.upsert(memory.id, memory.embedding, {
          type: memory.type,
          content: memory.content,
          importance: memory.importance,
          createdAt: memory.createdAt,
          updatedAt: memory.updatedAt,
          accessedAt: memory.accessedAt,
          accessCount: memory.accessCount,
          ...memory.metadata
        });
        count++;
      }
    }
    return count;
  }
  /**
   * Clear all memories
   */
  clear() {
    this.db.clear();
  }
  /**
   * Convert stored result to Memory object
   */
  resultToMemory(id, metadata) {
    if (!metadata) {
      return {
        id,
        type: "episodic",
        content: "",
        metadata: {},
        importance: 0,
        createdAt: Date.now(),
        updatedAt: Date.now(),
        accessedAt: Date.now(),
        accessCount: 0
      };
    }
    return {
      id,
      type: metadata.type || "episodic",
      content: metadata.content || "",
      metadata: Object.fromEntries(
        Object.entries(metadata).filter(
          ([k]) => !["type", "content", "importance", "createdAt", "updatedAt", "accessedAt", "accessCount"].includes(k)
        )
      ),
      importance: metadata.importance || 0,
      createdAt: metadata.createdAt || Date.now(),
      updatedAt: metadata.updatedAt || Date.now(),
      accessedAt: metadata.accessedAt || Date.now(),
      accessCount: metadata.accessCount || 0
    };
  }
};
__name(MemoryManager, "MemoryManager");

// src/storage/D1Storage.ts
var D1Storage = class {
  constructor(db) {
    this.db = db;
  }
  // ============ Namespace Operations ============
  async getNamespace(name) {
    const result = await this.db.prepare("SELECT * FROM namespaces WHERE name = ?").bind(name).first();
    if (!result)
      return null;
    return {
      name: result.name,
      dimensions: result.dimensions,
      createdAt: result.created_at,
      updatedAt: result.updated_at
    };
  }
  async createNamespace(name, dimensions) {
    const now = Date.now();
    await this.db.prepare("INSERT INTO namespaces (name, dimensions, created_at, updated_at) VALUES (?, ?, ?, ?)").bind(name, dimensions, now, now).run();
    return { name, dimensions, createdAt: now, updatedAt: now };
  }
  async listNamespaces() {
    const results = await this.db.prepare("SELECT * FROM namespaces ORDER BY name").all();
    return (results.results || []).map((r) => ({
      name: r.name,
      dimensions: r.dimensions,
      createdAt: r.created_at,
      updatedAt: r.updated_at
    }));
  }
  async deleteNamespace(name) {
    const result = await this.db.prepare("DELETE FROM namespaces WHERE name = ?").bind(name).run();
    return (result.meta?.changes ?? 0) > 0;
  }
  // ============ Memory Operations ============
  async saveMemory(memory) {
    const now = Date.now();
    await this.db.prepare(`
				INSERT OR REPLACE INTO memories
				(id, namespace, type, content, embedding, importance, metadata, session_id, ttl, created_at, updated_at, last_accessed, access_count)
				VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
			`).bind(
      memory.id,
      memory.namespace,
      memory.type,
      memory.content,
      JSON.stringify(memory.embedding),
      memory.importance,
      JSON.stringify(memory.metadata),
      memory.sessionId || null,
      memory.ttl || null,
      memory.createdAt,
      now,
      memory.lastAccessed || null,
      memory.accessCount
    ).run();
  }
  async getMemory(namespace, id) {
    const result = await this.db.prepare("SELECT * FROM memories WHERE namespace = ? AND id = ?").bind(namespace, id).first();
    if (!result)
      return null;
    await this.db.prepare("UPDATE memories SET access_count = access_count + 1, last_accessed = ? WHERE id = ?").bind(Date.now(), id).run();
    return this.rowToMemory(result);
  }
  async getAllMemories(namespace) {
    const results = await this.db.prepare("SELECT * FROM memories WHERE namespace = ? ORDER BY created_at DESC").bind(namespace).all();
    return (results.results || []).map((r) => this.rowToMemory(r));
  }
  async getMemoriesByType(namespace, type) {
    const results = await this.db.prepare("SELECT * FROM memories WHERE namespace = ? AND type = ? ORDER BY created_at DESC").bind(namespace, type).all();
    return (results.results || []).map((r) => this.rowToMemory(r));
  }
  async deleteMemory(namespace, id) {
    const result = await this.db.prepare("DELETE FROM memories WHERE namespace = ? AND id = ?").bind(namespace, id).run();
    return (result.meta?.changes ?? 0) > 0;
  }
  async deleteMemoriesByType(namespace, type) {
    const result = await this.db.prepare("DELETE FROM memories WHERE namespace = ? AND type = ?").bind(namespace, type).run();
    return result.meta?.changes ?? 0;
  }
  async clearNamespace(namespace) {
    const result = await this.db.prepare("DELETE FROM memories WHERE namespace = ?").bind(namespace).run();
    return result.meta?.changes ?? 0;
  }
  async updateMemory(namespace, id, updates) {
    const existing = await this.getMemory(namespace, id);
    if (!existing)
      return false;
    const filteredUpdates = Object.fromEntries(
      Object.entries(updates).filter(([, v]) => v !== void 0)
    );
    const updated = { ...existing, ...filteredUpdates, updatedAt: Date.now() };
    await this.saveMemory(updated);
    return true;
  }
  // ============ Stats ============
  async getStats(namespace) {
    const stats = await this.db.prepare(`
				SELECT
					COUNT(*) as total,
					SUM(CASE WHEN type = 'episodic' THEN 1 ELSE 0 END) as episodic,
					SUM(CASE WHEN type = 'semantic' THEN 1 ELSE 0 END) as semantic,
					SUM(CASE WHEN type = 'working' THEN 1 ELSE 0 END) as working,
					SUM(CASE WHEN type = 'knowledge' THEN 1 ELSE 0 END) as knowledge,
					AVG(importance) as avg_importance,
					MIN(created_at) as oldest,
					MAX(created_at) as newest
				FROM memories WHERE namespace = ?
			`).bind(namespace).first();
    const sourcesCount = await this.db.prepare("SELECT COUNT(*) as count FROM knowledge_sources WHERE namespace = ?").bind(namespace).first();
    return {
      total: stats?.total ?? 0,
      byType: {
        episodic: stats?.episodic ?? 0,
        semantic: stats?.semantic ?? 0,
        working: stats?.working ?? 0,
        knowledge: stats?.knowledge ?? 0
      },
      averageImportance: stats?.avg_importance ?? 0,
      oldestMemory: stats?.oldest ?? void 0,
      newestMemory: stats?.newest ?? void 0,
      knowledgeSources: sourcesCount?.count ?? 0
    };
  }
  // ============ Cleanup Operations ============
  async cleanupExpired(namespace) {
    const now = Date.now();
    const result = await this.db.prepare(`
				DELETE FROM memories
				WHERE namespace = ?
				AND type = 'working'
				AND ttl IS NOT NULL
				AND (created_at + ttl) < ?
			`).bind(namespace, now).run();
    return result.meta?.changes ?? 0;
  }
  async applyDecay(namespace, decayRate = 0.01) {
    const result = await this.db.prepare(`
				UPDATE memories
				SET importance = MAX(0.01, importance * (1 - ?)),
				    updated_at = ?
				WHERE namespace = ?
			`).bind(decayRate, Date.now(), namespace).run();
    return result.meta?.changes ?? 0;
  }
  // ============ API Key Operations ============
  async validateApiKey(key) {
    const result = await this.db.prepare("SELECT * FROM api_keys WHERE key = ? AND is_active = 1").bind(key).first();
    if (!result)
      return null;
    await this.db.prepare("UPDATE api_keys SET last_used = ? WHERE key = ?").bind(Date.now(), key).run();
    return {
      valid: true,
      userId: result.user_id,
      namespace: result.namespace || void 0,
      permissions: JSON.parse(result.permissions),
      rateLimit: {
        limit: result.rate_limit,
        window: result.rate_window
      }
    };
  }
  // ============ Helpers ============
  rowToMemory(row) {
    const metadata = row.metadata ? JSON.parse(row.metadata) : {};
    const memory = {
      id: row.id,
      namespace: row.namespace,
      type: row.type,
      content: row.content,
      embedding: JSON.parse(row.embedding),
      importance: row.importance,
      metadata,
      sessionId: row.session_id || void 0,
      ttl: row.ttl || void 0,
      createdAt: row.created_at,
      updatedAt: row.updated_at,
      lastAccessed: row.last_accessed || void 0,
      accessCount: row.access_count
    };
    if (row.type === "knowledge" && metadata) {
      memory.sourceId = metadata.sourceId;
      memory.sourceName = metadata.sourceName;
      memory.sourceType = metadata.sourceType;
      memory.chunkIndex = metadata.chunkIndex;
      memory.totalChunks = metadata.totalChunks;
    }
    return memory;
  }
};
__name(D1Storage, "D1Storage");

// src/services/EmbeddingService.ts
function normalizeVector(vector) {
  let norm = 0;
  for (const v of vector) {
    norm += v * v;
  }
  norm = Math.sqrt(norm);
  if (norm === 0)
    return vector;
  return vector.map((v) => v / norm);
}
__name(normalizeVector, "normalizeVector");
function truncateVector(vector, targetDims) {
  if (vector.length <= targetDims)
    return vector;
  return vector.slice(0, targetDims);
}
__name(truncateVector, "truncateVector");
var EmbeddingService = class {
  ai;
  model;
  defaultDimensions;
  constructor(ai, options) {
    this.ai = ai;
    this.model = options?.model || "@cf/google/gemma-embedding-300m";
    this.defaultDimensions = options?.defaultDimensions || 768;
  }
  /**
   * Generate embedding for a single text
   */
  async embed(text, options) {
    const dims = options?.dimensions || this.defaultDimensions;
    const shouldNormalize = options?.normalize ?? true;
    const response = await this.ai.run(this.model, {
      text: [text]
    });
    if (!response?.data?.[0]) {
      throw new Error("Failed to generate embedding: empty response");
    }
    let embedding = response.data[0];
    const originalDims = embedding.length;
    const truncated = dims < originalDims;
    if (truncated) {
      embedding = truncateVector(embedding, dims);
    }
    if (shouldNormalize) {
      embedding = normalizeVector(embedding);
    }
    return {
      embedding,
      dimensions: embedding.length,
      model: this.model,
      truncated
    };
  }
  /**
   * Generate embeddings for multiple texts (batch)
   */
  async embedBatch(texts, options) {
    const dims = options?.dimensions || this.defaultDimensions;
    const shouldNormalize = options?.normalize ?? true;
    const response = await this.ai.run(this.model, {
      text: texts
    });
    if (!response?.data || response.data.length === 0) {
      throw new Error("Failed to generate embeddings: empty response");
    }
    let embeddings = response.data;
    embeddings = embeddings.map((emb) => {
      let result = dims < emb.length ? truncateVector(emb, dims) : emb;
      if (shouldNormalize) {
        result = normalizeVector(result);
      }
      return result;
    });
    return {
      embeddings,
      dimensions: dims,
      model: this.model,
      count: embeddings.length
    };
  }
  /**
   * Get embedding for text, with caching support
   * Returns null if text is empty or only whitespace
   */
  async getEmbedding(text, options) {
    const trimmed = text?.trim();
    if (!trimmed) {
      return null;
    }
    const result = await this.embed(trimmed, options);
    return result.embedding;
  }
  /**
   * Get the configured default dimensions
   */
  get dimensions() {
    return this.defaultDimensions;
  }
  /**
   * Get the model name
   */
  get modelName() {
    return this.model;
  }
  /**
   * Estimate cost for N embeddings
   * $0.011 per 1000 neurons
   */
  static estimateCost(count, dimensions = 768) {
    const neurons = count * dimensions;
    return neurons / 1e3 * 0.011;
  }
  /**
   * Get daily free tier limit (in number of embeddings)
   * Free tier: 10,000 neurons/day
   */
  static getDailyFreeLimit(dimensions = 768) {
    return Math.floor(1e4 / dimensions);
  }
};
__name(EmbeddingService, "EmbeddingService");

// src/services/AuditService.ts
function generateAuditId() {
  const timestamp = Date.now().toString(36);
  const random = Math.random().toString(36).substring(2, 10);
  return `aud_${timestamp}_${random}`;
}
__name(generateAuditId, "generateAuditId");
function maskApiKey(apiKey) {
  if (!apiKey)
    return void 0;
  return apiKey.substring(0, 8);
}
__name(maskApiKey, "maskApiKey");
var AuditService = class {
  db;
  enabled;
  constructor(db, options) {
    this.db = db;
    this.enabled = options?.enabled ?? true;
  }
  /**
   * Log an audit entry
   */
  async log(options) {
    if (!this.enabled)
      return null;
    const id = generateAuditId();
    const timestamp = Date.now();
    try {
      await this.db.prepare(
        `INSERT INTO audit_log (
						id, timestamp, action, resource_type, resource_id,
						user_id, tenant_id, namespace, api_key_prefix,
						ip_address, user_agent, request_id,
						details, success, error_message, duration_ms
					) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)`
      ).bind(
        id,
        timestamp,
        options.action,
        options.resourceType,
        options.resourceId || null,
        options.userId || null,
        options.tenantId || null,
        options.namespace || null,
        maskApiKey(options.apiKey),
        options.ipAddress || null,
        options.userAgent || null,
        options.requestId || null,
        options.details ? JSON.stringify(options.details) : null,
        options.success !== false ? 1 : 0,
        options.errorMessage || null,
        options.durationMs || null
      ).run();
      return id;
    } catch (error) {
      console.error("Failed to write audit log:", error);
      return null;
    }
  }
  /**
   * Query audit logs
   */
  async query(options = {}) {
    const conditions = [];
    const params = [];
    if (options.action) {
      conditions.push("action = ?");
      params.push(options.action);
    }
    if (options.resourceType) {
      conditions.push("resource_type = ?");
      params.push(options.resourceType);
    }
    if (options.resourceId) {
      conditions.push("resource_id = ?");
      params.push(options.resourceId);
    }
    if (options.userId) {
      conditions.push("user_id = ?");
      params.push(options.userId);
    }
    if (options.tenantId) {
      conditions.push("tenant_id = ?");
      params.push(options.tenantId);
    }
    if (options.namespace) {
      conditions.push("namespace = ?");
      params.push(options.namespace);
    }
    if (options.startTime) {
      conditions.push("timestamp >= ?");
      params.push(options.startTime);
    }
    if (options.endTime) {
      conditions.push("timestamp <= ?");
      params.push(options.endTime);
    }
    if (options.success !== void 0) {
      conditions.push("success = ?");
      params.push(options.success ? 1 : 0);
    }
    if (options.requestId) {
      conditions.push("request_id = ?");
      params.push(options.requestId);
    }
    const whereClause = conditions.length > 0 ? `WHERE ${conditions.join(" AND ")}` : "";
    const limit = options.limit || 100;
    const offset = options.offset || 0;
    const countQuery = `SELECT COUNT(*) as total FROM audit_log ${whereClause}`;
    const countResult = await this.db.prepare(countQuery).bind(...params).first();
    const total = countResult?.total || 0;
    const query = `
			SELECT * FROM audit_log
			${whereClause}
			ORDER BY timestamp DESC
			LIMIT ? OFFSET ?
		`;
    const results = await this.db.prepare(query).bind(...params, limit, offset).all();
    const entries = (results.results || []).map((row) => ({
      id: row.id,
      timestamp: row.timestamp,
      action: row.action,
      resourceType: row.resource_type,
      resourceId: row.resource_id,
      userId: row.user_id,
      tenantId: row.tenant_id,
      namespace: row.namespace,
      apiKeyPrefix: row.api_key_prefix,
      ipAddress: row.ip_address,
      userAgent: row.user_agent,
      requestId: row.request_id,
      details: row.details ? JSON.parse(row.details) : void 0,
      success: row.success === 1,
      errorMessage: row.error_message,
      durationMs: row.duration_ms
    }));
    return {
      entries,
      total,
      hasMore: offset + entries.length < total
    };
  }
  /**
   * Get audit log by ID
   */
  async getById(id) {
    const row = await this.db.prepare("SELECT * FROM audit_log WHERE id = ?").bind(id).first();
    if (!row)
      return null;
    return {
      id: row.id,
      timestamp: row.timestamp,
      action: row.action,
      resourceType: row.resource_type,
      resourceId: row.resource_id,
      userId: row.user_id,
      tenantId: row.tenant_id,
      namespace: row.namespace,
      apiKeyPrefix: row.api_key_prefix,
      ipAddress: row.ip_address,
      userAgent: row.user_agent,
      requestId: row.request_id,
      details: row.details ? JSON.parse(row.details) : void 0,
      success: row.success === 1,
      errorMessage: row.error_message,
      durationMs: row.duration_ms
    };
  }
  /**
   * Get history for a specific resource
   */
  async getResourceHistory(resourceType, resourceId, limit = 50) {
    const result = await this.query({
      resourceType,
      resourceId,
      limit
    });
    return result.entries;
  }
  /**
   * Get activity for a specific user
   */
  async getUserActivity(userId, options) {
    const result = await this.query({
      userId,
      startTime: options?.startTime,
      endTime: options?.endTime,
      limit: options?.limit || 100
    });
    return result.entries;
  }
  /**
   * Get failed operations
   */
  async getFailures(options) {
    const result = await this.query({
      success: false,
      tenantId: options?.tenantId,
      namespace: options?.namespace,
      limit: options?.limit || 50
    });
    return result.entries;
  }
  /**
   * Get audit stats for a time period
   */
  async getStats(tenantId, options) {
    const conditions = [];
    const params = [];
    if (tenantId) {
      conditions.push("tenant_id = ?");
      params.push(tenantId);
    }
    if (options?.startTime) {
      conditions.push("timestamp >= ?");
      params.push(options.startTime);
    }
    if (options?.endTime) {
      conditions.push("timestamp <= ?");
      params.push(options.endTime);
    }
    const whereClause = conditions.length > 0 ? `WHERE ${conditions.join(" AND ")}` : "";
    const statsQuery = `
			SELECT
				COUNT(*) as total,
				SUM(CASE WHEN success = 1 THEN 1 ELSE 0 END) as successful,
				AVG(duration_ms) as avg_duration
			FROM audit_log ${whereClause}
		`;
    const stats = await this.db.prepare(statsQuery).bind(...params).first();
    const actionQuery = `
			SELECT action, COUNT(*) as count
			FROM audit_log ${whereClause}
			GROUP BY action
		`;
    const actionResults = await this.db.prepare(actionQuery).bind(...params).all();
    const byAction = {};
    for (const row of actionResults.results || []) {
      byAction[row.action] = row.count;
    }
    const resourceQuery = `
			SELECT resource_type, COUNT(*) as count
			FROM audit_log ${whereClause}
			GROUP BY resource_type
		`;
    const resourceResults = await this.db.prepare(resourceQuery).bind(...params).all();
    const byResource = {};
    for (const row of resourceResults.results || []) {
      byResource[row.resource_type] = row.count;
    }
    return {
      totalOperations: stats?.total || 0,
      byAction,
      byResource,
      successRate: stats?.total ? (stats.successful || 0) / stats.total * 100 : 100,
      avgDurationMs: stats?.avg_duration || 0
    };
  }
  /**
   * Clean up old audit logs
   */
  async cleanup(retentionDays = 90) {
    const cutoffTime = Date.now() - retentionDays * 24 * 60 * 60 * 1e3;
    const result = await this.db.prepare("DELETE FROM audit_log WHERE timestamp < ?").bind(cutoffTime).run();
    return result.meta.changes || 0;
  }
};
__name(AuditService, "AuditService");
function createAuditLogger(db, context) {
  const service = new AuditService(db);
  return {
    /**
     * Log a memory operation
     */
    async logMemory(action, memoryId, details, options) {
      return service.log({
        action,
        resourceType: "memory",
        resourceId: memoryId,
        ...context,
        details,
        ...options
      });
    },
    /**
     * Log a namespace operation
     */
    async logNamespace(action, namespaceName, details, options) {
      return service.log({
        action,
        resourceType: "namespace",
        resourceId: namespaceName,
        ...context,
        details,
        ...options
      });
    },
    /**
     * Log a bulk operation
     */
    async logBulk(action, details, options) {
      return service.log({
        action,
        resourceType: "memory",
        ...context,
        details,
        ...options
      });
    },
    /**
     * Log an auth operation
     */
    async logAuth(action, userId, details, options) {
      return service.log({
        action,
        resourceType: "user",
        resourceId: userId,
        ...context,
        userId,
        details,
        ...options
      });
    },
    /**
     * Access the underlying service for queries
     */
    service
  };
}
__name(createAuditLogger, "createAuditLogger");

// src/services/KnowledgeService.ts
var DEFAULT_CHUNK_SIZE = 1e3;
var DEFAULT_CHUNK_OVERLAP = 200;
var DEFAULT_SEPARATORS = ["\n\n", "\n", ". ", "! ", "? ", "; ", ", ", " "];
var KnowledgeService = class {
  constructor(db, config = {}) {
    this.db = db;
    this.config = {
      enabled: config.enabled ?? true,
      defaultChunkSize: config.defaultChunkSize ?? DEFAULT_CHUNK_SIZE,
      defaultChunkOverlap: config.defaultChunkOverlap ?? DEFAULT_CHUNK_OVERLAP,
      maxChunksPerDocument: config.maxChunksPerDocument ?? 1e3
    };
  }
  config;
  /**
   * Check if the service is available
   */
  isAvailable() {
    return this.config.enabled && this.db !== null;
  }
  /**
   * Chunk text into overlapping segments
   */
  chunkText(text, options = {}) {
    const chunkSize = options.chunkSize ?? this.config.defaultChunkSize;
    const overlap = options.chunkOverlap ?? this.config.defaultChunkOverlap;
    const separators = options.separators ?? DEFAULT_SEPARATORS;
    const preserveParagraphs = options.preserveParagraphs ?? true;
    const chunks = [];
    let currentPosition = 0;
    let chunkIndex = 0;
    while (currentPosition < text.length) {
      let endPosition = Math.min(currentPosition + chunkSize, text.length);
      if (endPosition < text.length) {
        const searchStart = Math.max(currentPosition + chunkSize - 100, currentPosition);
        let bestBreak = -1;
        for (const separator of separators) {
          const idx = text.lastIndexOf(separator, endPosition);
          if (idx > searchStart) {
            bestBreak = idx + separator.length;
            if (preserveParagraphs && separator === "\n\n") {
              break;
            }
          }
        }
        if (bestBreak > currentPosition) {
          endPosition = bestBreak;
        }
      }
      const chunkText = text.slice(currentPosition, endPosition).trim();
      if (chunkText.length > 0) {
        chunks.push({
          text: chunkText,
          index: chunkIndex,
          startOffset: currentPosition,
          endOffset: endPosition
        });
        chunkIndex++;
      }
      currentPosition = endPosition - overlap;
      if (currentPosition <= chunks[chunks.length - 1]?.startOffset) {
        currentPosition = endPosition;
      }
      if (chunks.length >= this.config.maxChunksPerDocument) {
        break;
      }
    }
    return chunks;
  }
  /**
   * Create a new knowledge source
   */
  async createSource(namespace, source) {
    if (!this.db) {
      throw new Error("D1 database not configured");
    }
    const now = Date.now();
    const id = `src_${generateId2()}`;
    const knowledgeSource = {
      id,
      ...source,
      createdAt: now,
      updatedAt: now
    };
    await this.db.prepare(`
				INSERT INTO knowledge_sources
				(id, namespace, name, type, url, mime_type, size, chunk_count, metadata, created_at, updated_at)
				VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
			`).bind(
      knowledgeSource.id,
      namespace,
      knowledgeSource.name,
      knowledgeSource.type,
      knowledgeSource.url || null,
      knowledgeSource.mimeType || null,
      knowledgeSource.size || null,
      knowledgeSource.chunkCount,
      JSON.stringify(knowledgeSource.metadata),
      knowledgeSource.createdAt,
      knowledgeSource.updatedAt
    ).run();
    return knowledgeSource;
  }
  /**
   * Get a knowledge source by ID
   */
  async getSource(id) {
    if (!this.db)
      return null;
    const result = await this.db.prepare("SELECT * FROM knowledge_sources WHERE id = ?").bind(id).first();
    if (!result)
      return null;
    return this.rowToSource(result);
  }
  /**
   * List all knowledge sources in a namespace
   */
  async listSources(namespace, options) {
    if (!this.db) {
      return { sources: [], total: 0 };
    }
    const limit = options?.limit ?? 100;
    const offset = options?.offset ?? 0;
    let query = "SELECT * FROM knowledge_sources WHERE namespace = ?";
    const params = [namespace];
    if (options?.type) {
      query += " AND type = ?";
      params.push(options.type);
    }
    query += " ORDER BY created_at DESC LIMIT ? OFFSET ?";
    params.push(limit, offset);
    const [results, countResult] = await Promise.all([
      this.db.prepare(query).bind(...params).all(),
      this.db.prepare("SELECT COUNT(*) as total FROM knowledge_sources WHERE namespace = ?").bind(namespace).first()
    ]);
    return {
      sources: (results.results || []).map((r) => this.rowToSource(r)),
      total: countResult?.total ?? 0
    };
  }
  /**
   * Delete a knowledge source and all its chunks
   */
  async deleteSource(id) {
    if (!this.db)
      return false;
    await this.db.prepare("DELETE FROM memories WHERE metadata LIKE ?").bind(`%"sourceId":"${id}"%`).run();
    const result = await this.db.prepare("DELETE FROM knowledge_sources WHERE id = ?").bind(id).run();
    return (result.meta?.changes ?? 0) > 0;
  }
  /**
   * Update chunk count for a source
   */
  async updateSourceChunkCount(id, chunkCount) {
    if (!this.db)
      return;
    await this.db.prepare("UPDATE knowledge_sources SET chunk_count = ?, updated_at = ? WHERE id = ?").bind(chunkCount, Date.now(), id).run();
  }
  /**
   * Get knowledge stats for a namespace
   */
  async getStats(namespace) {
    if (!this.db) {
      return {
        totalSources: 0,
        totalChunks: 0,
        byType: { document: 0, url: 0, api: 0, manual: 0 },
        totalSize: 0
      };
    }
    const stats = await this.db.prepare(`
				SELECT
					COUNT(*) as total_sources,
					SUM(chunk_count) as total_chunks,
					SUM(CASE WHEN type = 'document' THEN 1 ELSE 0 END) as documents,
					SUM(CASE WHEN type = 'url' THEN 1 ELSE 0 END) as urls,
					SUM(CASE WHEN type = 'api' THEN 1 ELSE 0 END) as apis,
					SUM(CASE WHEN type = 'manual' THEN 1 ELSE 0 END) as manuals,
					SUM(COALESCE(size, 0)) as total_size
				FROM knowledge_sources WHERE namespace = ?
			`).bind(namespace).first();
    return {
      totalSources: stats?.total_sources ?? 0,
      totalChunks: stats?.total_chunks ?? 0,
      byType: {
        document: stats?.documents ?? 0,
        url: stats?.urls ?? 0,
        api: stats?.apis ?? 0,
        manual: stats?.manuals ?? 0
      },
      totalSize: stats?.total_size ?? 0
    };
  }
  // ============ Helper Methods ============
  rowToSource(row) {
    return {
      id: row.id,
      name: row.name,
      type: row.type,
      url: row.url || void 0,
      mimeType: row.mime_type || void 0,
      size: row.size || void 0,
      chunkCount: row.chunk_count,
      namespace: row.namespace,
      metadata: row.metadata ? JSON.parse(row.metadata) : {},
      createdAt: row.created_at,
      updatedAt: row.updated_at
    };
  }
};
__name(KnowledgeService, "KnowledgeService");
function generateId2() {
  const timestamp = Date.now().toString(36);
  const random = Math.random().toString(36).substring(2, 8);
  return `${timestamp}_${random}`;
}
__name(generateId2, "generateId");

// src/services/WebhookService.ts
var WEBHOOK_EVENTS = [
  "memory.remembered",
  "memory.forgotten",
  "memory.updated",
  "knowledge.ingested",
  "knowledge.deleted"
];
function generateWebhookId() {
  const timestamp = Date.now().toString(36);
  const random = Math.random().toString(36).substring(2, 10);
  return `wh_${timestamp}_${random}`;
}
__name(generateWebhookId, "generateWebhookId");
function generateDeliveryId() {
  const timestamp = Date.now().toString(36);
  const random = Math.random().toString(36).substring(2, 10);
  return `del_${timestamp}_${random}`;
}
__name(generateDeliveryId, "generateDeliveryId");
function generateEventId() {
  const timestamp = Date.now().toString(36);
  const random = Math.random().toString(36).substring(2, 10);
  return `evt_${timestamp}_${random}`;
}
__name(generateEventId, "generateEventId");
function generateSecret() {
  const array = new Uint8Array(32);
  crypto.getRandomValues(array);
  return Array.from(array).map((b) => b.toString(16).padStart(2, "0")).join("");
}
__name(generateSecret, "generateSecret");
async function signPayload(payload, secret) {
  const encoder2 = new TextEncoder();
  const keyData = encoder2.encode(secret);
  const payloadData = encoder2.encode(payload);
  const key = await crypto.subtle.importKey("raw", keyData, { name: "HMAC", hash: "SHA-256" }, false, ["sign"]);
  const signature = await crypto.subtle.sign("HMAC", key, payloadData);
  const signatureArray = new Uint8Array(signature);
  return Array.from(signatureArray).map((b) => b.toString(16).padStart(2, "0")).join("");
}
__name(signPayload, "signPayload");
var WebhookService = class {
  db;
  ctx;
  constructor(db, ctx) {
    this.db = db;
    this.ctx = ctx;
  }
  /**
   * Create a new webhook
   */
  async create(options) {
    const id = generateWebhookId();
    const secret = generateSecret();
    const now = Date.now();
    const webhook = {
      id,
      namespace: options.namespace,
      tenantId: options.tenantId,
      url: options.url,
      secret,
      events: options.events,
      isActive: true,
      description: options.description,
      maxRetries: options.maxRetries ?? 3,
      retryBackoffMs: options.retryBackoffMs ?? 1e3,
      successCount: 0,
      failureCount: 0,
      createdAt: now,
      updatedAt: now
    };
    await this.db.prepare(
      `INSERT INTO webhooks (
					id, namespace, tenant_id, url, secret, events,
					is_active, description, max_retries, retry_backoff_ms,
					success_count, failure_count, created_at, updated_at
				) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)`
    ).bind(
      webhook.id,
      webhook.namespace,
      webhook.tenantId || null,
      webhook.url,
      webhook.secret,
      JSON.stringify(webhook.events),
      webhook.isActive ? 1 : 0,
      webhook.description || null,
      webhook.maxRetries,
      webhook.retryBackoffMs,
      webhook.successCount,
      webhook.failureCount,
      webhook.createdAt,
      webhook.updatedAt
    ).run();
    return webhook;
  }
  /**
   * Get a webhook by ID
   */
  async get(id) {
    const row = await this.db.prepare("SELECT * FROM webhooks WHERE id = ?").bind(id).first();
    if (!row)
      return null;
    return this.rowToWebhook(row);
  }
  /**
   * List webhooks for a namespace
   */
  async list(namespace, options) {
    const conditions = ["namespace = ?"];
    const params = [namespace];
    if (options?.activeOnly) {
      conditions.push("is_active = 1");
    }
    const whereClause = conditions.join(" AND ");
    const limit = options?.limit ?? 100;
    const offset = options?.offset ?? 0;
    const countResult = await this.db.prepare(`SELECT COUNT(*) as total FROM webhooks WHERE ${whereClause}`).bind(...params).first();
    const total = countResult?.total ?? 0;
    const results = await this.db.prepare(
      `SELECT * FROM webhooks WHERE ${whereClause}
				ORDER BY created_at DESC LIMIT ? OFFSET ?`
    ).bind(...params, limit, offset).all();
    const webhooks = (results.results || []).map((row) => this.rowToWebhook(row));
    return {
      webhooks,
      total,
      hasMore: offset + webhooks.length < total
    };
  }
  /**
   * Update a webhook
   */
  async update(id, updates) {
    const webhook = await this.get(id);
    if (!webhook)
      return null;
    const now = Date.now();
    const fields = ["updated_at = ?"];
    const params = [now];
    if (updates.url !== void 0) {
      fields.push("url = ?");
      params.push(updates.url);
    }
    if (updates.events !== void 0) {
      fields.push("events = ?");
      params.push(JSON.stringify(updates.events));
    }
    if (updates.isActive !== void 0) {
      fields.push("is_active = ?");
      params.push(updates.isActive ? 1 : 0);
    }
    if (updates.description !== void 0) {
      fields.push("description = ?");
      params.push(updates.description || null);
    }
    if (updates.maxRetries !== void 0) {
      fields.push("max_retries = ?");
      params.push(updates.maxRetries);
    }
    if (updates.retryBackoffMs !== void 0) {
      fields.push("retry_backoff_ms = ?");
      params.push(updates.retryBackoffMs);
    }
    params.push(id);
    await this.db.prepare(`UPDATE webhooks SET ${fields.join(", ")} WHERE id = ?`).bind(...params).run();
    return this.get(id);
  }
  /**
   * Delete a webhook
   */
  async delete(id) {
    const result = await this.db.prepare("DELETE FROM webhooks WHERE id = ?").bind(id).run();
    return (result.meta.changes || 0) > 0;
  }
  /**
   * Rotate the secret for a webhook
   */
  async rotateSecret(id) {
    const webhook = await this.get(id);
    if (!webhook)
      return null;
    const newSecret = generateSecret();
    const now = Date.now();
    await this.db.prepare("UPDATE webhooks SET secret = ?, updated_at = ? WHERE id = ?").bind(newSecret, now, id).run();
    return newSecret;
  }
  /**
   * Trigger webhooks for an event (async, non-blocking)
   */
  async trigger(namespace, eventType, data, tenantId) {
    const result = await this.db.prepare("SELECT * FROM webhooks WHERE namespace = ? AND is_active = 1").bind(namespace).all();
    const webhooks = (result.results || []).map((row) => this.rowToWebhook(row)).filter((wh) => wh.events.includes(eventType));
    if (webhooks.length === 0)
      return;
    const event = {
      id: generateEventId(),
      type: eventType,
      timestamp: Date.now(),
      namespace,
      tenantId,
      data
    };
    const payloadString = JSON.stringify(event);
    for (const webhook of webhooks) {
      const deliveryId = generateDeliveryId();
      await this.db.prepare(
        `INSERT INTO webhook_deliveries (
						id, webhook_id, event_type, event_id, payload, status, attempt_count, created_at
					) VALUES (?, ?, ?, ?, ?, ?, ?, ?)`
      ).bind(deliveryId, webhook.id, eventType, event.id, payloadString, "pending", 0, Date.now()).run();
      if (this.ctx) {
        this.ctx.waitUntil(this.deliver(webhook, deliveryId, payloadString));
      } else {
        this.deliver(webhook, deliveryId, payloadString).catch(console.error);
      }
    }
    const webhookIds = webhooks.map((wh) => wh.id);
    await this.db.prepare(
      `UPDATE webhooks SET last_triggered_at = ? WHERE id IN (${webhookIds.map(() => "?").join(", ")})`
    ).bind(Date.now(), ...webhookIds).run();
  }
  /**
   * Deliver a webhook event
   */
  async deliver(webhook, deliveryId, payload) {
    let attemptCount = 0;
    let lastError;
    let responseStatus;
    let responseBody;
    while (attemptCount <= webhook.maxRetries) {
      attemptCount++;
      await this.db.prepare("UPDATE webhook_deliveries SET status = ?, attempt_count = ? WHERE id = ?").bind(attemptCount === 1 ? "pending" : "retrying", attemptCount, deliveryId).run();
      try {
        const signature = await signPayload(payload, webhook.secret);
        const response = await fetch(webhook.url, {
          method: "POST",
          headers: {
            "Content-Type": "application/json",
            "X-Signature": `sha256=${signature}`,
            "X-Webhook-Id": webhook.id,
            "X-Event-Id": JSON.parse(payload).id,
            "X-Event-Type": JSON.parse(payload).type,
            "User-Agent": "minimemory-webhooks/1.0"
          },
          body: payload
        });
        responseStatus = response.status;
        responseBody = await response.text().catch(() => "");
        if (response.ok) {
          await this.db.prepare(
            `UPDATE webhook_deliveries
							SET status = 'success', response_status = ?, response_body = ?, completed_at = ?
							WHERE id = ?`
          ).bind(responseStatus, responseBody.substring(0, 1e3), Date.now(), deliveryId).run();
          await this.db.prepare("UPDATE webhooks SET success_count = success_count + 1 WHERE id = ?").bind(webhook.id).run();
          return;
        }
        lastError = `HTTP ${response.status}: ${responseBody.substring(0, 200)}`;
      } catch (error) {
        lastError = error instanceof Error ? error.message : "Unknown error";
      }
      if (attemptCount <= webhook.maxRetries) {
        const backoff = webhook.retryBackoffMs * Math.pow(2, attemptCount - 1);
        const nextRetryAt = Date.now() + backoff;
        await this.db.prepare("UPDATE webhook_deliveries SET next_retry_at = ? WHERE id = ?").bind(nextRetryAt, deliveryId).run();
        await new Promise((resolve) => setTimeout(resolve, backoff));
      }
    }
    await this.db.prepare(
      `UPDATE webhook_deliveries
				SET status = 'failed', response_status = ?, error_message = ?, completed_at = ?
				WHERE id = ?`
    ).bind(responseStatus || null, lastError || "Unknown error", Date.now(), deliveryId).run();
    await this.db.prepare("UPDATE webhooks SET failure_count = failure_count + 1 WHERE id = ?").bind(webhook.id).run();
  }
  /**
   * Test a webhook by sending a test event
   */
  async test(id) {
    const webhook = await this.get(id);
    if (!webhook) {
      return { success: false, error: "Webhook not found" };
    }
    const testEvent = {
      id: generateEventId(),
      type: "memory.remembered",
      timestamp: Date.now(),
      namespace: webhook.namespace,
      tenantId: webhook.tenantId,
      data: {
        test: true,
        message: "This is a test webhook delivery"
      }
    };
    const payload = JSON.stringify(testEvent);
    try {
      const signature = await signPayload(payload, webhook.secret);
      const response = await fetch(webhook.url, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          "X-Signature": `sha256=${signature}`,
          "X-Webhook-Id": webhook.id,
          "X-Event-Id": testEvent.id,
          "X-Event-Type": testEvent.type,
          "User-Agent": "minimemory-webhooks/1.0 (test)"
        },
        body: payload
      });
      if (response.ok) {
        return { success: true, status: response.status };
      }
      return {
        success: false,
        status: response.status,
        error: `HTTP ${response.status}`
      };
    } catch (error) {
      return {
        success: false,
        error: error instanceof Error ? error.message : "Unknown error"
      };
    }
  }
  /**
   * Get delivery history for a webhook
   */
  async getDeliveries(options = {}) {
    const conditions = [];
    const params = [];
    if (options.webhookId) {
      conditions.push("webhook_id = ?");
      params.push(options.webhookId);
    }
    if (options.status) {
      conditions.push("status = ?");
      params.push(options.status);
    }
    if (options.eventType) {
      conditions.push("event_type = ?");
      params.push(options.eventType);
    }
    const whereClause = conditions.length > 0 ? `WHERE ${conditions.join(" AND ")}` : "";
    const limit = options.limit ?? 100;
    const offset = options.offset ?? 0;
    const countResult = await this.db.prepare(`SELECT COUNT(*) as total FROM webhook_deliveries ${whereClause}`).bind(...params).first();
    const total = countResult?.total ?? 0;
    const results = await this.db.prepare(
      `SELECT * FROM webhook_deliveries ${whereClause}
				ORDER BY created_at DESC LIMIT ? OFFSET ?`
    ).bind(...params, limit, offset).all();
    const deliveries = (results.results || []).map((row) => this.rowToDelivery(row));
    return {
      deliveries,
      total,
      hasMore: offset + deliveries.length < total
    };
  }
  /**
   * Cleanup old deliveries
   */
  async cleanupDeliveries(retentionDays = 7) {
    const cutoffTime = Date.now() - retentionDays * 24 * 60 * 60 * 1e3;
    const result = await this.db.prepare("DELETE FROM webhook_deliveries WHERE created_at < ? AND status IN (?, ?)").bind(cutoffTime, "success", "failed").run();
    return result.meta.changes || 0;
  }
  /**
   * Convert a database row to a Webhook object
   */
  rowToWebhook(row) {
    return {
      id: row.id,
      namespace: row.namespace,
      tenantId: row.tenant_id,
      url: row.url,
      secret: row.secret,
      events: JSON.parse(row.events),
      isActive: row.is_active === 1,
      description: row.description,
      maxRetries: row.max_retries,
      retryBackoffMs: row.retry_backoff_ms,
      successCount: row.success_count,
      failureCount: row.failure_count,
      lastTriggeredAt: row.last_triggered_at,
      createdAt: row.created_at,
      updatedAt: row.updated_at
    };
  }
  /**
   * Convert a database row to a WebhookDelivery object
   */
  rowToDelivery(row) {
    return {
      id: row.id,
      webhookId: row.webhook_id,
      eventType: row.event_type,
      eventId: row.event_id,
      payload: row.payload,
      status: row.status,
      attemptCount: row.attempt_count,
      nextRetryAt: row.next_retry_at,
      responseStatus: row.response_status,
      responseBody: row.response_body,
      errorMessage: row.error_message,
      createdAt: row.created_at,
      completedAt: row.completed_at
    };
  }
};
__name(WebhookService, "WebhookService");
function createWebhookTrigger(db, ctx) {
  const service = new WebhookService(db, ctx);
  return (namespace, event, data, tenantId) => {
    service.trigger(namespace, event, data, tenantId).catch((error) => {
      console.error("Webhook trigger failed:", error);
    });
  };
}
__name(createWebhookTrigger, "createWebhookTrigger");

// src/services/AgentTokenService.ts
function generateAgentTokenId() {
  const timestamp = Date.now().toString(36);
  const random = Math.random().toString(36).substring(2, 10);
  return `at_${timestamp}_${random}`;
}
__name(generateAgentTokenId, "generateAgentTokenId");
var AgentTokenService = class {
  db;
  constructor(db) {
    this.db = db;
  }
  /**
   * Create a new agent token
   */
  async create(options) {
    const id = generateAgentTokenId();
    const now = Date.now();
    const token = {
      id,
      userId: options.userId,
      tenantId: options.tenantId,
      name: options.name,
      description: options.description,
      allowedMemories: options.allowedMemories ?? ["*"],
      permissions: options.permissions ?? ["read", "write"],
      isActive: true,
      useCount: 0,
      expiresAt: options.expiresAt,
      createdAt: now,
      updatedAt: now
    };
    await this.db.prepare(
      `INSERT INTO agent_tokens (
					id, user_id, tenant_id, name, description,
					allowed_memories, permissions, is_active,
					use_count, expires_at, created_at, updated_at
				) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)`
    ).bind(
      token.id,
      token.userId,
      token.tenantId || null,
      token.name,
      token.description || null,
      JSON.stringify(token.allowedMemories),
      JSON.stringify(token.permissions),
      token.isActive ? 1 : 0,
      token.useCount,
      token.expiresAt || null,
      token.createdAt,
      token.updatedAt
    ).run();
    return token;
  }
  /**
   * Get an agent token by ID
   */
  async get(id) {
    const row = await this.db.prepare("SELECT * FROM agent_tokens WHERE id = ?").bind(id).first();
    if (!row)
      return null;
    return this.rowToToken(row);
  }
  /**
   * Get an agent token by ID and verify ownership
   */
  async getByIdAndUser(id, userId) {
    const row = await this.db.prepare("SELECT * FROM agent_tokens WHERE id = ? AND user_id = ?").bind(id, userId).first();
    if (!row)
      return null;
    return this.rowToToken(row);
  }
  /**
   * List agent tokens
   */
  async list(options = {}) {
    const conditions = [];
    const params = [];
    if (options.userId) {
      conditions.push("user_id = ?");
      params.push(options.userId);
    }
    if (options.tenantId) {
      conditions.push("tenant_id = ?");
      params.push(options.tenantId);
    }
    if (options.activeOnly) {
      conditions.push("is_active = 1");
    }
    const whereClause = conditions.length > 0 ? `WHERE ${conditions.join(" AND ")}` : "";
    const limit = options.limit ?? 100;
    const offset = options.offset ?? 0;
    const countResult = await this.db.prepare(`SELECT COUNT(*) as total FROM agent_tokens ${whereClause}`).bind(...params).first();
    const total = countResult?.total ?? 0;
    const results = await this.db.prepare(
      `SELECT * FROM agent_tokens ${whereClause}
				ORDER BY created_at DESC LIMIT ? OFFSET ?`
    ).bind(...params, limit, offset).all();
    const tokens = (results.results || []).map((row) => this.rowToToken(row));
    return {
      tokens,
      total,
      hasMore: offset + tokens.length < total
    };
  }
  /**
   * Update an agent token
   */
  async update(id, updates) {
    const token = await this.get(id);
    if (!token)
      return null;
    const now = Date.now();
    const fields = ["updated_at = ?"];
    const params = [now];
    if (updates.name !== void 0) {
      fields.push("name = ?");
      params.push(updates.name);
    }
    if (updates.description !== void 0) {
      fields.push("description = ?");
      params.push(updates.description || null);
    }
    if (updates.allowedMemories !== void 0) {
      fields.push("allowed_memories = ?");
      params.push(JSON.stringify(updates.allowedMemories));
    }
    if (updates.permissions !== void 0) {
      fields.push("permissions = ?");
      params.push(JSON.stringify(updates.permissions));
    }
    if (updates.isActive !== void 0) {
      fields.push("is_active = ?");
      params.push(updates.isActive ? 1 : 0);
    }
    if (updates.expiresAt !== void 0) {
      fields.push("expires_at = ?");
      params.push(updates.expiresAt || null);
    }
    params.push(id);
    await this.db.prepare(`UPDATE agent_tokens SET ${fields.join(", ")} WHERE id = ?`).bind(...params).run();
    return this.get(id);
  }
  /**
   * Delete an agent token
   */
  async delete(id) {
    const result = await this.db.prepare("DELETE FROM agent_tokens WHERE id = ?").bind(id).run();
    return (result.meta.changes || 0) > 0;
  }
  /**
   * Toggle agent token active status
   */
  async toggle(id) {
    const token = await this.get(id);
    if (!token)
      return null;
    return this.update(id, { isActive: !token.isActive });
  }
  /**
   * Add a memory to the allowed list
   */
  async addAllowedMemory(id, memoryId) {
    const token = await this.get(id);
    if (!token)
      return null;
    if (token.allowedMemories.includes("*")) {
      return token;
    }
    if (token.allowedMemories.includes(memoryId)) {
      return token;
    }
    const newAllowed = [...token.allowedMemories, memoryId];
    return this.update(id, { allowedMemories: newAllowed });
  }
  /**
   * Remove a memory from the allowed list
   */
  async removeAllowedMemory(id, memoryId) {
    const token = await this.get(id);
    if (!token)
      return null;
    const newAllowed = token.allowedMemories.filter((m) => m !== memoryId);
    if (newAllowed.length === 0) {
      return null;
    }
    return this.update(id, { allowedMemories: newAllowed });
  }
  /**
   * Validate API key and agent token for MCP authentication
   */
  async validate(apiKey, agentTokenId) {
    const apiKeyRow = await this.db.prepare(
      `SELECT ak.*, u.id as uid, u.email, u.is_active as user_active
				FROM api_keys ak
				LEFT JOIN users u ON ak.user_id = u.id
				WHERE ak.key = ? AND ak.is_active = 1`
    ).bind(apiKey).first();
    if (!apiKeyRow) {
      return { valid: false, error: "Invalid API key" };
    }
    const userId = apiKeyRow.user_id;
    if (userId && apiKeyRow.user_active === 0) {
      return { valid: false, error: "User account is inactive" };
    }
    const tokenRow = await this.db.prepare("SELECT * FROM agent_tokens WHERE id = ?").bind(agentTokenId).first();
    if (!tokenRow) {
      return { valid: false, error: "Invalid agent token" };
    }
    const token = this.rowToToken(tokenRow);
    if (userId && token.userId !== userId) {
      return { valid: false, error: "Agent token does not belong to this user" };
    }
    if (!token.isActive) {
      return { valid: false, error: "Agent token is inactive" };
    }
    if (token.expiresAt && token.expiresAt < Date.now()) {
      return { valid: false, error: "Agent token has expired" };
    }
    return {
      valid: true,
      userId: token.userId,
      tenantId: token.tenantId,
      agentTokenId: token.id,
      agentName: token.name,
      allowedMemories: token.allowedMemories,
      permissions: token.permissions,
      expiresAt: token.expiresAt
    };
  }
  /**
   * Record token usage
   */
  async recordUsage(id) {
    const now = Date.now();
    await this.db.prepare("UPDATE agent_tokens SET last_used_at = ?, use_count = use_count + 1 WHERE id = ?").bind(now, id).run();
  }
  /**
   * Check if a token can access a specific memory
   */
  canAccessMemory(token, memoryId) {
    const allowedMemories = "allowedMemories" in token ? token.allowedMemories : [];
    if (!allowedMemories)
      return false;
    if (allowedMemories.includes("*"))
      return true;
    return allowedMemories.includes(memoryId);
  }
  /**
   * Check if a token has a specific permission
   */
  hasPermission(token, permission) {
    const permissions = "permissions" in token ? token.permissions : [];
    if (!permissions)
      return false;
    return permissions.includes(permission);
  }
  /**
   * Filter a list of memory IDs to only those the token can access
   */
  filterAllowedMemories(token, memoryIds) {
    const allowedMemories = "allowedMemories" in token ? token.allowedMemories : [];
    if (!allowedMemories)
      return [];
    if (allowedMemories.includes("*"))
      return memoryIds;
    return memoryIds.filter((id) => allowedMemories.includes(id));
  }
  /**
   * Get usage statistics for a user's tokens
   */
  async getStats(userId) {
    const now = Date.now();
    const result = await this.db.prepare(
      `SELECT
					COUNT(*) as total,
					SUM(CASE WHEN is_active = 1 AND (expires_at IS NULL OR expires_at > ?) THEN 1 ELSE 0 END) as active,
					SUM(CASE WHEN is_active = 0 THEN 1 ELSE 0 END) as inactive,
					SUM(CASE WHEN expires_at IS NOT NULL AND expires_at <= ? THEN 1 ELSE 0 END) as expired,
					SUM(use_count) as total_use_count
				FROM agent_tokens WHERE user_id = ?`
    ).bind(now, now, userId).first();
    return {
      total: result?.total || 0,
      active: result?.active || 0,
      inactive: result?.inactive || 0,
      expired: result?.expired || 0,
      totalUseCount: result?.total_use_count || 0
    };
  }
  /**
   * Cleanup expired tokens (optional - tokens can be kept for audit)
   */
  async cleanupExpired() {
    const now = Date.now();
    const cutoff = now - 30 * 24 * 60 * 60 * 1e3;
    const result = await this.db.prepare("DELETE FROM agent_tokens WHERE expires_at IS NOT NULL AND expires_at < ?").bind(cutoff).run();
    return result.meta.changes || 0;
  }
  /**
   * Convert a database row to an AgentToken object
   */
  rowToToken(row) {
    return {
      id: row.id,
      userId: row.user_id,
      tenantId: row.tenant_id,
      name: row.name,
      description: row.description,
      allowedMemories: JSON.parse(row.allowed_memories),
      permissions: JSON.parse(row.permissions),
      isActive: row.is_active === 1,
      lastUsedAt: row.last_used_at,
      useCount: row.use_count,
      expiresAt: row.expires_at,
      createdAt: row.created_at,
      updatedAt: row.updated_at
    };
  }
};
__name(AgentTokenService, "AgentTokenService");

// src/api/memory.ts
var DEFAULT_EMBEDDING_DIMS = 768;
function createMemoryRoutes(getManager2) {
  const api = new Hono2();
  function getStorage(c) {
    return c.env?.DB ? new D1Storage(c.env.DB) : null;
  }
  __name(getStorage, "getStorage");
  function getAuditContext2(c) {
    return {
      userId: c.req.header("X-User-Id"),
      tenantId: c.req.header("X-Tenant-Id"),
      apiKey: c.req.header("X-API-Key"),
      ipAddress: c.req.header("CF-Connecting-IP") || c.req.header("X-Forwarded-For"),
      userAgent: c.req.header("User-Agent"),
      requestId: c.req.header("X-Request-Id") || crypto.randomUUID()
    };
  }
  __name(getAuditContext2, "getAuditContext");
  function getAuditLogger(c, namespace) {
    if (!c.env?.DB)
      return null;
    const context = getAuditContext2(c);
    return createAuditLogger(c.env.DB, { ...context, namespace });
  }
  __name(getAuditLogger, "getAuditLogger");
  function getWebhookTrigger(c) {
    if (!c.env?.DB)
      return null;
    return createWebhookTrigger(c.env.DB, c.executionCtx);
  }
  __name(getWebhookTrigger, "getWebhookTrigger");
  function getEmbeddingService(c, dimensions) {
    if (!c.env?.AI)
      return null;
    return new EmbeddingService(c.env.AI, {
      defaultDimensions: dimensions || DEFAULT_EMBEDDING_DIMS
    });
  }
  __name(getEmbeddingService, "getEmbeddingService");
  async function getNamespaceDimensions(storage, namespace) {
    if (!storage)
      return void 0;
    const ns = await storage.getNamespace(namespace);
    return ns?.dimensions;
  }
  __name(getNamespaceDimensions, "getNamespaceDimensions");
  api.post("/remember", async (c) => {
    const startTime = Date.now();
    const namespace = c.req.header("X-Namespace") || "default";
    const auditLogger = getAuditLogger(c, namespace);
    try {
      const body = await c.req.json();
      const storage = getStorage(c);
      const { content, embedding, type, importance, metadata, sessionId, ttl, generateEmbedding } = body;
      if (!content || typeof content !== "string") {
        return c.json({ error: "content is required and must be a string" }, 400);
      }
      const namespaceDims = await getNamespaceDimensions(storage, namespace);
      const dimensions = namespaceDims || DEFAULT_EMBEDDING_DIMS;
      let vectorEmbedding;
      let embeddingGenerated = false;
      if (embedding && Array.isArray(embedding) && embedding.length > 0) {
        vectorEmbedding = embedding;
      } else if (generateEmbedding !== false) {
        const embeddingService = getEmbeddingService(c, dimensions);
        if (embeddingService) {
          try {
            const result = await embeddingService.embed(content, {
              dimensions
            });
            vectorEmbedding = result.embedding;
            embeddingGenerated = true;
          } catch (embError) {
            console.error("Failed to generate embedding:", embError);
            vectorEmbedding = new Array(dimensions).fill(0);
          }
        } else {
          vectorEmbedding = new Array(dimensions).fill(0);
        }
      } else {
        vectorEmbedding = new Array(dimensions).fill(0);
      }
      const manager = getManager2(namespace, dimensions);
      const options = { type, importance, metadata, sessionId, ttl };
      const memory = await manager.remember(content, vectorEmbedding, options);
      if (storage) {
        await storage.saveMemory({
          id: memory.id,
          namespace,
          type: memory.type,
          content: memory.content,
          embedding: vectorEmbedding,
          importance: memory.importance,
          metadata: memory.metadata || {},
          sessionId,
          ttl,
          createdAt: memory.createdAt,
          updatedAt: memory.createdAt,
          accessCount: 0
        });
      }
      await auditLogger?.logMemory("create", memory.id, {
        type: memory.type,
        importance: memory.importance,
        embeddingGenerated,
        contentLength: content.length
      }, { durationMs: Date.now() - startTime });
      const tenantId = c.req.header("X-Tenant-Id");
      const webhookTrigger = getWebhookTrigger(c);
      webhookTrigger?.(namespace, "memory.remembered", {
        memoryId: memory.id,
        type: memory.type,
        content: memory.content,
        importance: memory.importance,
        metadata: memory.metadata
      }, tenantId);
      return c.json({
        success: true,
        memory: {
          id: memory.id,
          type: memory.type,
          content: memory.content,
          importance: memory.importance,
          createdAt: memory.createdAt
        },
        embeddingGenerated,
        persisted: !!storage
      });
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : "Unknown error";
      await auditLogger?.logMemory("create", void 0, { error: errorMessage }, {
        success: false,
        errorMessage,
        durationMs: Date.now() - startTime
      });
      return c.json({ error: errorMessage }, 500);
    }
  });
  api.post("/recall", async (c) => {
    const startTime = Date.now();
    const namespace = c.req.header("X-Namespace") || "default";
    const auditLogger = getAuditLogger(c, namespace);
    try {
      const body = await c.req.json();
      const storage = getStorage(c);
      const { query, keywords, embedding, type, limit, minImportance, minSimilarity, sessionId, mode, alpha } = body;
      const namespaceDims = await getNamespaceDimensions(storage, namespace);
      const dimensions = namespaceDims || DEFAULT_EMBEDDING_DIMS;
      let queryEmbedding = embedding;
      let embeddingGenerated = false;
      if (!embedding && query) {
        const embeddingService = getEmbeddingService(c, dimensions);
        if (embeddingService) {
          try {
            const result = await embeddingService.embed(query, {
              dimensions
            });
            queryEmbedding = result.embedding;
            embeddingGenerated = true;
          } catch (embError) {
            console.error("Failed to generate query embedding:", embError);
          }
        }
      }
      const searchKeywords = keywords || query;
      if (!searchKeywords && !queryEmbedding) {
        return c.json({ error: "Either query, keywords, or embedding is required" }, 400);
      }
      const manager = getManager2(namespace, dimensions);
      const currentStats = await manager.stats();
      if (storage && currentStats.total === 0) {
        const storedMemories = await storage.getAllMemories(namespace);
        if (storedMemories.length > 0) {
          manager.import({
            memories: storedMemories.map((m) => ({
              id: m.id,
              type: m.type,
              content: m.content,
              embedding: m.embedding,
              importance: m.importance,
              metadata: m.metadata,
              createdAt: m.createdAt,
              updatedAt: m.updatedAt,
              accessedAt: m.lastAccessed || m.createdAt,
              accessCount: m.accessCount
            }))
          });
        }
      }
      const options = {
        type,
        limit,
        minImportance,
        minSimilarity,
        sessionId,
        mode,
        alpha
      };
      let results;
      if (searchKeywords && queryEmbedding) {
        results = await manager.recallWithEmbedding(searchKeywords, queryEmbedding, options);
      } else if (queryEmbedding) {
        options.mode = "vector";
        results = await manager.recall(queryEmbedding, options);
      } else {
        options.mode = "keyword";
        results = await manager.recall(searchKeywords, options);
      }
      await auditLogger?.logMemory("search", void 0, {
        mode: mode || (queryEmbedding && searchKeywords ? "hybrid" : queryEmbedding ? "vector" : "keyword"),
        resultCount: results.length,
        embeddingGenerated,
        limit
      }, { durationMs: Date.now() - startTime });
      const mappedResults = results.map((r) => {
        const result = {
          id: r.memory.id,
          type: r.memory.type,
          content: r.memory.content,
          score: r.score,
          vectorSimilarity: r.vectorSimilarity,
          keywordScore: r.keywordScore,
          importance: r.memory.importance,
          metadata: r.memory.metadata,
          createdAt: r.memory.createdAt
        };
        if (r.memory.type === "knowledge" && r.memory.metadata) {
          const meta = r.memory.metadata;
          if (meta.sourceId) {
            result.source = {
              id: meta.sourceId,
              name: meta.sourceName,
              type: meta.sourceType,
              url: meta.sourceUrl,
              chunkIndex: meta.chunkIndex,
              totalChunks: meta.totalChunks
            };
          }
        }
        return result;
      });
      return c.json({
        success: true,
        count: results.length,
        embeddingGenerated,
        results: mappedResults
      });
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : "Unknown error";
      await auditLogger?.logMemory("search", void 0, { error: errorMessage }, {
        success: false,
        errorMessage,
        durationMs: Date.now() - startTime
      });
      return c.json({ error: errorMessage }, 500);
    }
  });
  api.delete("/forget/:id", async (c) => {
    const startTime = Date.now();
    const id = c.req.param("id");
    const namespace = c.req.header("X-Namespace") || "default";
    const auditLogger = getAuditLogger(c, namespace);
    try {
      const storage = getStorage(c);
      const manager = getManager2(namespace);
      const deleted = await manager.forget(id);
      if (storage) {
        await storage.deleteMemory(namespace, id);
      }
      await auditLogger?.logMemory("delete", id, { deleted }, { durationMs: Date.now() - startTime });
      if (deleted) {
        const tenantId = c.req.header("X-Tenant-Id");
        const webhookTrigger = getWebhookTrigger(c);
        webhookTrigger?.(namespace, "memory.forgotten", {
          memoryId: id
        }, tenantId);
      }
      return c.json({
        success: deleted,
        message: deleted ? "Memory forgotten" : "Memory not found"
      });
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : "Unknown error";
      await auditLogger?.logMemory("delete", id, { error: errorMessage }, {
        success: false,
        errorMessage,
        durationMs: Date.now() - startTime
      });
      return c.json({ error: errorMessage }, 500);
    }
  });
  api.post("/forget", async (c) => {
    const startTime = Date.now();
    const namespace = c.req.header("X-Namespace") || "default";
    const auditLogger = getAuditLogger(c, namespace);
    try {
      const body = await c.req.json();
      const storage = getStorage(c);
      const { filter } = body;
      if (!filter || typeof filter !== "object") {
        return c.json({ error: "filter is required and must be an object" }, 400);
      }
      const manager = getManager2(namespace);
      const count = await manager.forgetByFilter(filter);
      if (storage && filter.type) {
        await storage.deleteMemoriesByType(namespace, filter.type);
      }
      await auditLogger?.logMemory("delete", void 0, { filter, count }, { durationMs: Date.now() - startTime });
      return c.json({
        success: true,
        count,
        message: `Forgot ${count} memories`
      });
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : "Unknown error";
      await auditLogger?.logMemory("delete", void 0, { error: errorMessage }, {
        success: false,
        errorMessage,
        durationMs: Date.now() - startTime
      });
      return c.json({ error: errorMessage }, 500);
    }
  });
  api.get("/memory/:id", async (c) => {
    const startTime = Date.now();
    const id = c.req.param("id");
    const namespace = c.req.header("X-Namespace") || "default";
    const auditLogger = getAuditLogger(c, namespace);
    try {
      const storage = getStorage(c);
      if (storage) {
        const stored = await storage.getMemory(namespace, id);
        if (stored) {
          await auditLogger?.logMemory("read", id, { source: "d1" }, { durationMs: Date.now() - startTime });
          return c.json({
            success: true,
            memory: {
              id: stored.id,
              type: stored.type,
              content: stored.content,
              importance: stored.importance,
              metadata: stored.metadata,
              createdAt: stored.createdAt,
              lastAccessed: stored.lastAccessed,
              accessCount: stored.accessCount
            },
            source: "d1"
          });
        }
      }
      const manager = getManager2(namespace);
      const memory = await manager.get(id);
      if (!memory) {
        return c.json({ error: "Memory not found" }, 404);
      }
      await auditLogger?.logMemory("read", id, { source: "memory" }, { durationMs: Date.now() - startTime });
      return c.json({
        success: true,
        memory,
        source: "memory"
      });
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : "Unknown error";
      await auditLogger?.logMemory("read", id, { error: errorMessage }, {
        success: false,
        errorMessage,
        durationMs: Date.now() - startTime
      });
      return c.json({ error: errorMessage }, 500);
    }
  });
  api.patch("/memory/:id", async (c) => {
    const startTime = Date.now();
    const id = c.req.param("id");
    const namespace = c.req.header("X-Namespace") || "default";
    const auditLogger = getAuditLogger(c, namespace);
    try {
      const body = await c.req.json();
      const storage = getStorage(c);
      const { content, importance, metadata, embedding } = body;
      const manager = getManager2(namespace);
      const memory = await manager.update(id, { content, importance, metadata }, embedding);
      if (!memory) {
        return c.json({ error: "Memory not found" }, 404);
      }
      if (storage) {
        await storage.updateMemory(namespace, id, {
          content: memory.content,
          importance: memory.importance,
          metadata: memory.metadata,
          embedding
        });
      }
      const updatedFields = Object.keys(body).filter((k) => body[k] !== void 0);
      await auditLogger?.logMemory("update", id, {
        updatedFields
      }, { durationMs: Date.now() - startTime });
      const tenantId = c.req.header("X-Tenant-Id");
      const webhookTrigger = getWebhookTrigger(c);
      webhookTrigger?.(namespace, "memory.updated", {
        memoryId: id,
        updatedFields,
        content: memory.content,
        importance: memory.importance,
        metadata: memory.metadata
      }, tenantId);
      return c.json({
        success: true,
        memory
      });
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : "Unknown error";
      await auditLogger?.logMemory("update", id, { error: errorMessage }, {
        success: false,
        errorMessage,
        durationMs: Date.now() - startTime
      });
      return c.json({ error: errorMessage }, 500);
    }
  });
  api.get("/stats", async (c) => {
    try {
      const namespace = c.req.header("X-Namespace") || "default";
      const storage = getStorage(c);
      if (storage) {
        const stats2 = await storage.getStats(namespace);
        return c.json({
          success: true,
          namespace,
          stats: stats2,
          source: "d1"
        });
      }
      const manager = getManager2(namespace);
      const stats = await manager.stats();
      return c.json({
        success: true,
        namespace,
        stats,
        source: "memory"
      });
    } catch (error) {
      return c.json({
        error: error instanceof Error ? error.message : "Unknown error"
      }, 500);
    }
  });
  api.post("/cleanup", async (c) => {
    try {
      const namespace = c.req.header("X-Namespace") || "default";
      const storage = getStorage(c);
      const manager = getManager2(namespace);
      let count = await manager.cleanupExpired();
      if (storage) {
        const d1Count = await storage.cleanupExpired(namespace);
        count = Math.max(count, d1Count);
      }
      return c.json({
        success: true,
        count,
        message: `Cleaned up ${count} expired memories`
      });
    } catch (error) {
      return c.json({
        error: error instanceof Error ? error.message : "Unknown error"
      }, 500);
    }
  });
  api.post("/decay", async (c) => {
    try {
      const namespace = c.req.header("X-Namespace") || "default";
      const storage = getStorage(c);
      const manager = getManager2(namespace);
      await manager.applyDecay();
      if (storage) {
        await storage.applyDecay(namespace);
      }
      return c.json({
        success: true,
        message: "Decay applied successfully"
      });
    } catch (error) {
      return c.json({
        error: error instanceof Error ? error.message : "Unknown error"
      }, 500);
    }
  });
  api.post("/export", async (c) => {
    const startTime = Date.now();
    const namespace = c.req.header("X-Namespace") || "default";
    const auditLogger = getAuditLogger(c, namespace);
    try {
      const storage = getStorage(c);
      if (storage) {
        const memories = await storage.getAllMemories(namespace);
        await auditLogger?.logBulk("export", {
          count: memories.length,
          source: "d1"
        }, { durationMs: Date.now() - startTime });
        return c.json({
          success: true,
          namespace,
          data: { memories },
          source: "d1"
        });
      }
      const manager = getManager2(namespace);
      const data = manager.export();
      await auditLogger?.logBulk("export", {
        count: data.memories?.length || 0,
        source: "memory"
      }, { durationMs: Date.now() - startTime });
      return c.json({
        success: true,
        namespace,
        data,
        source: "memory"
      });
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : "Unknown error";
      await auditLogger?.logBulk("export", { error: errorMessage }, {
        success: false,
        errorMessage,
        durationMs: Date.now() - startTime
      });
      return c.json({ error: errorMessage }, 500);
    }
  });
  api.post("/import", async (c) => {
    const startTime = Date.now();
    const namespace = c.req.header("X-Namespace") || "default";
    const auditLogger = getAuditLogger(c, namespace);
    try {
      const body = await c.req.json();
      const storage = getStorage(c);
      const { memories } = body;
      if (!memories || !Array.isArray(memories)) {
        return c.json({ error: "memories array is required" }, 400);
      }
      const manager = getManager2(namespace);
      const count = manager.import({ memories });
      if (storage) {
        for (const mem of memories) {
          await storage.saveMemory({
            id: mem.id,
            namespace,
            type: mem.type || "semantic",
            content: mem.content,
            embedding: mem.embedding,
            importance: mem.importance || 0.5,
            metadata: mem.metadata || {},
            sessionId: mem.sessionId,
            ttl: mem.ttl,
            createdAt: mem.createdAt || Date.now(),
            updatedAt: Date.now(),
            accessCount: 0
          });
        }
      }
      await auditLogger?.logBulk("import", { count }, { durationMs: Date.now() - startTime });
      return c.json({
        success: true,
        count,
        message: `Imported ${count} memories`
      });
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : "Unknown error";
      await auditLogger?.logBulk("import", { error: errorMessage }, {
        success: false,
        errorMessage,
        durationMs: Date.now() - startTime
      });
      return c.json({ error: errorMessage }, 500);
    }
  });
  api.delete("/clear", async (c) => {
    const startTime = Date.now();
    const namespace = c.req.header("X-Namespace") || "default";
    const auditLogger = getAuditLogger(c, namespace);
    try {
      const storage = getStorage(c);
      const manager = getManager2(namespace);
      manager.clear();
      if (storage) {
        await storage.clearNamespace(namespace);
      }
      await auditLogger?.logBulk("clear", { namespace }, { durationMs: Date.now() - startTime });
      return c.json({
        success: true,
        message: "All memories cleared"
      });
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : "Unknown error";
      await auditLogger?.logBulk("clear", { error: errorMessage }, {
        success: false,
        errorMessage,
        durationMs: Date.now() - startTime
      });
      return c.json({ error: errorMessage }, 500);
    }
  });
  api.post("/embed", async (c) => {
    try {
      const body = await c.req.json();
      const { text, texts, dimensions } = body;
      const inputTexts = texts || (text ? [text] : null);
      if (!inputTexts || !Array.isArray(inputTexts) || inputTexts.length === 0) {
        return c.json({ error: "text or texts array is required" }, 400);
      }
      const targetDims = dimensions || DEFAULT_EMBEDDING_DIMS;
      if (![768, 512, 256, 128].includes(targetDims)) {
        return c.json({ error: "dimensions must be one of: 768, 512, 256, 128" }, 400);
      }
      const embeddingService = getEmbeddingService(c, targetDims);
      if (!embeddingService) {
        return c.json({ error: "AI binding not available. Configure AI in wrangler.toml" }, 503);
      }
      if (inputTexts.length === 1) {
        const result = await embeddingService.embed(inputTexts[0], { dimensions: targetDims });
        return c.json({
          success: true,
          embedding: result.embedding,
          dimensions: result.dimensions,
          model: result.model,
          truncated: result.truncated
        });
      } else {
        const result = await embeddingService.embedBatch(inputTexts, { dimensions: targetDims });
        return c.json({
          success: true,
          embeddings: result.embeddings,
          dimensions: result.dimensions,
          model: result.model,
          count: result.count
        });
      }
    } catch (error) {
      return c.json({
        error: error instanceof Error ? error.message : "Unknown error"
      }, 500);
    }
  });
  api.get("/embed/info", (c) => {
    const hasAI = !!c.env?.AI;
    return c.json({
      available: hasAI,
      model: "@cf/google/gemma-embedding-300m",
      dimensions: {
        default: 768,
        available: [768, 512, 256, 128]
      },
      matryoshka: true,
      pricing: {
        perThousandNeurons: 0.011,
        freeDaily: 1e4
      },
      estimatedCosts: {
        "10K embeddings (768d)": EmbeddingService.estimateCost(1e4, 768).toFixed(2),
        "10K embeddings (256d)": EmbeddingService.estimateCost(1e4, 256).toFixed(2)
      }
    });
  });
  return api;
}
__name(createMemoryRoutes, "createMemoryRoutes");

// src/middleware/auth.ts
var ApiKeyStore = class {
  keys = /* @__PURE__ */ new Map();
  /**
   * Add an API key
   */
  addKey(apiKey, config) {
    this.keys.set(apiKey, { valid: true, ...config });
  }
  /**
   * Remove an API key
   */
  removeKey(apiKey) {
    return this.keys.delete(apiKey);
  }
  /**
   * Validate an API key
   */
  async validate(apiKey) {
    const result = this.keys.get(apiKey);
    return result || null;
  }
  /**
   * List all keys (for admin)
   */
  listKeys() {
    return Array.from(this.keys.keys());
  }
  /**
   * Generate a new API key
   */
  static generateKey() {
    const chars = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let key = "mm_";
    for (let i = 0; i < 32; i++) {
      key += chars.charAt(Math.floor(Math.random() * chars.length));
    }
    return key;
  }
};
__name(ApiKeyStore, "ApiKeyStore");
var defaultKeyStore = new ApiKeyStore();
defaultKeyStore.addKey("mm_dev_key_12345", {
  userId: "dev",
  namespace: "default",
  permissions: ["read", "write", "admin"],
  rateLimit: { limit: 1e3, window: 60 }
});
function createAuthMiddleware(config) {
  const headerName = config?.headerName || "X-API-Key";
  const queryParam = config?.queryParam || "api_key";
  const publicPaths = config?.publicPaths || ["/", "/health"];
  const validateKey = config?.validateKey || ((key) => defaultKeyStore.validate(key));
  return async (c, next) => {
    const path = c.req.path;
    if (publicPaths.some((p) => path === p || path.startsWith(p + "/"))) {
      await next();
      return;
    }
    const authMethod = c.get("authMethod");
    if (authMethod === "jwt") {
      await next();
      return;
    }
    const apiKey = c.req.header(headerName) || c.req.query(queryParam);
    if (!apiKey) {
      return c.json({
        error: "Unauthorized",
        message: `API key required. Provide via ${headerName} header or ${queryParam} query parameter.`
      }, 401);
    }
    const authResult = await validateKey(apiKey);
    if (!authResult || !authResult.valid) {
      return c.json({
        error: "Unauthorized",
        message: "Invalid API key"
      }, 401);
    }
    c.set("auth", authResult);
    c.set("userId", authResult.userId || null);
    c.set("authMethod", "apikey");
    if (authResult.namespace && !c.req.header("X-Namespace")) {
      c.set("namespace", authResult.namespace);
    }
    await next();
  };
}
__name(createAuthMiddleware, "createAuthMiddleware");

// src/middleware/rateLimit.ts
var RateLimiter = class {
  entries = /* @__PURE__ */ new Map();
  lastCleanup = 0;
  cleanupInterval = null;
  constructor() {
  }
  /**
   * Check and increment rate limit
   */
  check(key, limit, windowSeconds) {
    const now = Date.now();
    const windowMs = windowSeconds * 1e3;
    const resetAt = now + windowMs;
    if (now - this.lastCleanup > 6e4) {
      this.cleanup();
      this.lastCleanup = now;
    }
    let entry = this.entries.get(key);
    if (!entry || entry.resetAt <= now) {
      entry = { count: 1, resetAt };
      this.entries.set(key, entry);
      return {
        limit,
        remaining: limit - 1,
        reset: Math.floor(resetAt / 1e3),
        retryAfter: 0
      };
    }
    entry.count++;
    const remaining = Math.max(0, limit - entry.count);
    const retryAfter = entry.count > limit ? Math.ceil((entry.resetAt - now) / 1e3) : 0;
    return {
      limit,
      remaining,
      reset: Math.floor(entry.resetAt / 1e3),
      retryAfter
    };
  }
  /**
   * Clean up expired entries
   */
  cleanup() {
    const now = Date.now();
    for (const [key, entry] of this.entries) {
      if (entry.resetAt <= now) {
        this.entries.delete(key);
      }
    }
  }
  /**
   * Reset a specific key
   */
  reset(key) {
    this.entries.delete(key);
  }
  /**
   * Get current stats
   */
  stats() {
    return { activeKeys: this.entries.size };
  }
  /**
   * Start periodic cleanup (for Node.js environments)
   * Not needed for Workers - they are stateless
   */
  startCleanup() {
    if (!this.cleanupInterval && typeof setInterval !== "undefined") {
      this.cleanupInterval = setInterval(() => this.cleanup(), 6e4);
    }
  }
  /**
   * Destroy the rate limiter
   */
  destroy() {
    if (this.cleanupInterval) {
      clearInterval(this.cleanupInterval);
      this.cleanupInterval = null;
    }
    this.entries.clear();
  }
};
__name(RateLimiter, "RateLimiter");
var defaultRateLimiter = new RateLimiter();
function createRateLimitMiddleware(config) {
  const defaultLimit = config?.defaultLimit || 100;
  const defaultWindow = config?.defaultWindow || 60;
  const keyGenerator = config?.keyGenerator || defaultKeyGenerator;
  const skip = config?.skip || (() => false);
  const onLimit = config?.onLimit || defaultOnLimit;
  return async (c, next) => {
    if (skip(c)) {
      await next();
      return;
    }
    const auth2 = c.get("auth");
    const limit = auth2?.rateLimit?.limit || defaultLimit;
    const window = auth2?.rateLimit?.window || defaultWindow;
    const key = keyGenerator(c);
    const info = defaultRateLimiter.check(key, limit, window);
    c.header("X-RateLimit-Limit", String(info.limit));
    c.header("X-RateLimit-Remaining", String(info.remaining));
    c.header("X-RateLimit-Reset", String(info.reset));
    if (info.retryAfter > 0) {
      c.header("Retry-After", String(info.retryAfter));
      return onLimit(c, info);
    }
    await next();
  };
}
__name(createRateLimitMiddleware, "createRateLimitMiddleware");
function defaultKeyGenerator(c) {
  const auth2 = c.get("auth");
  if (auth2?.userId) {
    return `user:${auth2.userId}`;
  }
  const ip = c.req.header("CF-Connecting-IP") || c.req.header("X-Forwarded-For")?.split(",")[0] || "unknown";
  return `ip:${ip}`;
}
__name(defaultKeyGenerator, "defaultKeyGenerator");
function defaultOnLimit(c, info) {
  return c.json({
    error: "Too Many Requests",
    message: `Rate limit exceeded. Try again in ${info.retryAfter} seconds.`,
    retryAfter: info.retryAfter
  }, 429);
}
__name(defaultOnLimit, "defaultOnLimit");

// node_modules/jose/dist/webapi/lib/buffer_utils.js
var encoder = new TextEncoder();
var decoder = new TextDecoder();
var MAX_INT32 = 2 ** 32;
function concat(...buffers) {
  const size = buffers.reduce((acc, { length }) => acc + length, 0);
  const buf = new Uint8Array(size);
  let i = 0;
  for (const buffer of buffers) {
    buf.set(buffer, i);
    i += buffer.length;
  }
  return buf;
}
__name(concat, "concat");
function encode(string) {
  const bytes = new Uint8Array(string.length);
  for (let i = 0; i < string.length; i++) {
    const code = string.charCodeAt(i);
    if (code > 127) {
      throw new TypeError("non-ASCII string encountered in encode()");
    }
    bytes[i] = code;
  }
  return bytes;
}
__name(encode, "encode");

// node_modules/jose/dist/webapi/lib/base64.js
function encodeBase64(input) {
  if (Uint8Array.prototype.toBase64) {
    return input.toBase64();
  }
  const CHUNK_SIZE = 32768;
  const arr = [];
  for (let i = 0; i < input.length; i += CHUNK_SIZE) {
    arr.push(String.fromCharCode.apply(null, input.subarray(i, i + CHUNK_SIZE)));
  }
  return btoa(arr.join(""));
}
__name(encodeBase64, "encodeBase64");
function decodeBase64(encoded) {
  if (Uint8Array.fromBase64) {
    return Uint8Array.fromBase64(encoded);
  }
  const binary = atob(encoded);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i++) {
    bytes[i] = binary.charCodeAt(i);
  }
  return bytes;
}
__name(decodeBase64, "decodeBase64");

// node_modules/jose/dist/webapi/util/base64url.js
function decode(input) {
  if (Uint8Array.fromBase64) {
    return Uint8Array.fromBase64(typeof input === "string" ? input : decoder.decode(input), {
      alphabet: "base64url"
    });
  }
  let encoded = input;
  if (encoded instanceof Uint8Array) {
    encoded = decoder.decode(encoded);
  }
  encoded = encoded.replace(/-/g, "+").replace(/_/g, "/");
  try {
    return decodeBase64(encoded);
  } catch {
    throw new TypeError("The input to be decoded is not correctly encoded.");
  }
}
__name(decode, "decode");
function encode2(input) {
  let unencoded = input;
  if (typeof unencoded === "string") {
    unencoded = encoder.encode(unencoded);
  }
  if (Uint8Array.prototype.toBase64) {
    return unencoded.toBase64({ alphabet: "base64url", omitPadding: true });
  }
  return encodeBase64(unencoded).replace(/=/g, "").replace(/\+/g, "-").replace(/\//g, "_");
}
__name(encode2, "encode");

// node_modules/jose/dist/webapi/util/errors.js
var JOSEError = class extends Error {
  code = "ERR_JOSE_GENERIC";
  constructor(message2, options) {
    super(message2, options);
    this.name = this.constructor.name;
    Error.captureStackTrace?.(this, this.constructor);
  }
};
__name(JOSEError, "JOSEError");
__publicField(JOSEError, "code", "ERR_JOSE_GENERIC");
var JWTClaimValidationFailed = class extends JOSEError {
  code = "ERR_JWT_CLAIM_VALIDATION_FAILED";
  claim;
  reason;
  payload;
  constructor(message2, payload, claim = "unspecified", reason = "unspecified") {
    super(message2, { cause: { claim, reason, payload } });
    this.claim = claim;
    this.reason = reason;
    this.payload = payload;
  }
};
__name(JWTClaimValidationFailed, "JWTClaimValidationFailed");
__publicField(JWTClaimValidationFailed, "code", "ERR_JWT_CLAIM_VALIDATION_FAILED");
var JWTExpired = class extends JOSEError {
  code = "ERR_JWT_EXPIRED";
  claim;
  reason;
  payload;
  constructor(message2, payload, claim = "unspecified", reason = "unspecified") {
    super(message2, { cause: { claim, reason, payload } });
    this.claim = claim;
    this.reason = reason;
    this.payload = payload;
  }
};
__name(JWTExpired, "JWTExpired");
__publicField(JWTExpired, "code", "ERR_JWT_EXPIRED");
var JOSEAlgNotAllowed = class extends JOSEError {
  code = "ERR_JOSE_ALG_NOT_ALLOWED";
};
__name(JOSEAlgNotAllowed, "JOSEAlgNotAllowed");
__publicField(JOSEAlgNotAllowed, "code", "ERR_JOSE_ALG_NOT_ALLOWED");
var JOSENotSupported = class extends JOSEError {
  code = "ERR_JOSE_NOT_SUPPORTED";
};
__name(JOSENotSupported, "JOSENotSupported");
__publicField(JOSENotSupported, "code", "ERR_JOSE_NOT_SUPPORTED");
var JWSInvalid = class extends JOSEError {
  code = "ERR_JWS_INVALID";
};
__name(JWSInvalid, "JWSInvalid");
__publicField(JWSInvalid, "code", "ERR_JWS_INVALID");
var JWTInvalid = class extends JOSEError {
  code = "ERR_JWT_INVALID";
};
__name(JWTInvalid, "JWTInvalid");
__publicField(JWTInvalid, "code", "ERR_JWT_INVALID");
var JWKSMultipleMatchingKeys = class extends JOSEError {
  [Symbol.asyncIterator];
  code = "ERR_JWKS_MULTIPLE_MATCHING_KEYS";
  constructor(message2 = "multiple matching keys found in the JSON Web Key Set", options) {
    super(message2, options);
  }
};
__name(JWKSMultipleMatchingKeys, "JWKSMultipleMatchingKeys");
__publicField(JWKSMultipleMatchingKeys, "code", "ERR_JWKS_MULTIPLE_MATCHING_KEYS");
var JWSSignatureVerificationFailed = class extends JOSEError {
  code = "ERR_JWS_SIGNATURE_VERIFICATION_FAILED";
  constructor(message2 = "signature verification failed", options) {
    super(message2, options);
  }
};
__name(JWSSignatureVerificationFailed, "JWSSignatureVerificationFailed");
__publicField(JWSSignatureVerificationFailed, "code", "ERR_JWS_SIGNATURE_VERIFICATION_FAILED");

// node_modules/jose/dist/webapi/lib/crypto_key.js
var unusable = /* @__PURE__ */ __name((name, prop = "algorithm.name") => new TypeError(`CryptoKey does not support this operation, its ${prop} must be ${name}`), "unusable");
var isAlgorithm = /* @__PURE__ */ __name((algorithm, name) => algorithm.name === name, "isAlgorithm");
function getHashLength(hash) {
  return parseInt(hash.name.slice(4), 10);
}
__name(getHashLength, "getHashLength");
function getNamedCurve(alg) {
  switch (alg) {
    case "ES256":
      return "P-256";
    case "ES384":
      return "P-384";
    case "ES512":
      return "P-521";
    default:
      throw new Error("unreachable");
  }
}
__name(getNamedCurve, "getNamedCurve");
function checkUsage(key, usage) {
  if (usage && !key.usages.includes(usage)) {
    throw new TypeError(`CryptoKey does not support this operation, its usages must include ${usage}.`);
  }
}
__name(checkUsage, "checkUsage");
function checkSigCryptoKey(key, alg, usage) {
  switch (alg) {
    case "HS256":
    case "HS384":
    case "HS512": {
      if (!isAlgorithm(key.algorithm, "HMAC"))
        throw unusable("HMAC");
      const expected = parseInt(alg.slice(2), 10);
      const actual = getHashLength(key.algorithm.hash);
      if (actual !== expected)
        throw unusable(`SHA-${expected}`, "algorithm.hash");
      break;
    }
    case "RS256":
    case "RS384":
    case "RS512": {
      if (!isAlgorithm(key.algorithm, "RSASSA-PKCS1-v1_5"))
        throw unusable("RSASSA-PKCS1-v1_5");
      const expected = parseInt(alg.slice(2), 10);
      const actual = getHashLength(key.algorithm.hash);
      if (actual !== expected)
        throw unusable(`SHA-${expected}`, "algorithm.hash");
      break;
    }
    case "PS256":
    case "PS384":
    case "PS512": {
      if (!isAlgorithm(key.algorithm, "RSA-PSS"))
        throw unusable("RSA-PSS");
      const expected = parseInt(alg.slice(2), 10);
      const actual = getHashLength(key.algorithm.hash);
      if (actual !== expected)
        throw unusable(`SHA-${expected}`, "algorithm.hash");
      break;
    }
    case "Ed25519":
    case "EdDSA": {
      if (!isAlgorithm(key.algorithm, "Ed25519"))
        throw unusable("Ed25519");
      break;
    }
    case "ML-DSA-44":
    case "ML-DSA-65":
    case "ML-DSA-87": {
      if (!isAlgorithm(key.algorithm, alg))
        throw unusable(alg);
      break;
    }
    case "ES256":
    case "ES384":
    case "ES512": {
      if (!isAlgorithm(key.algorithm, "ECDSA"))
        throw unusable("ECDSA");
      const expected = getNamedCurve(alg);
      const actual = key.algorithm.namedCurve;
      if (actual !== expected)
        throw unusable(expected, "algorithm.namedCurve");
      break;
    }
    default:
      throw new TypeError("CryptoKey does not support this operation");
  }
  checkUsage(key, usage);
}
__name(checkSigCryptoKey, "checkSigCryptoKey");

// node_modules/jose/dist/webapi/lib/invalid_key_input.js
function message(msg, actual, ...types) {
  types = types.filter(Boolean);
  if (types.length > 2) {
    const last = types.pop();
    msg += `one of type ${types.join(", ")}, or ${last}.`;
  } else if (types.length === 2) {
    msg += `one of type ${types[0]} or ${types[1]}.`;
  } else {
    msg += `of type ${types[0]}.`;
  }
  if (actual == null) {
    msg += ` Received ${actual}`;
  } else if (typeof actual === "function" && actual.name) {
    msg += ` Received function ${actual.name}`;
  } else if (typeof actual === "object" && actual != null) {
    if (actual.constructor?.name) {
      msg += ` Received an instance of ${actual.constructor.name}`;
    }
  }
  return msg;
}
__name(message, "message");
var invalidKeyInput = /* @__PURE__ */ __name((actual, ...types) => message("Key must be ", actual, ...types), "invalidKeyInput");
var withAlg = /* @__PURE__ */ __name((alg, actual, ...types) => message(`Key for the ${alg} algorithm must be `, actual, ...types), "withAlg");

// node_modules/jose/dist/webapi/lib/is_key_like.js
var isCryptoKey = /* @__PURE__ */ __name((key) => {
  if (key?.[Symbol.toStringTag] === "CryptoKey")
    return true;
  try {
    return key instanceof CryptoKey;
  } catch {
    return false;
  }
}, "isCryptoKey");
var isKeyObject = /* @__PURE__ */ __name((key) => key?.[Symbol.toStringTag] === "KeyObject", "isKeyObject");
var isKeyLike = /* @__PURE__ */ __name((key) => isCryptoKey(key) || isKeyObject(key), "isKeyLike");

// node_modules/jose/dist/webapi/lib/is_disjoint.js
function isDisjoint(...headers) {
  const sources = headers.filter(Boolean);
  if (sources.length === 0 || sources.length === 1) {
    return true;
  }
  let acc;
  for (const header of sources) {
    const parameters = Object.keys(header);
    if (!acc || acc.size === 0) {
      acc = new Set(parameters);
      continue;
    }
    for (const parameter of parameters) {
      if (acc.has(parameter)) {
        return false;
      }
      acc.add(parameter);
    }
  }
  return true;
}
__name(isDisjoint, "isDisjoint");

// node_modules/jose/dist/webapi/lib/is_object.js
var isObjectLike = /* @__PURE__ */ __name((value) => typeof value === "object" && value !== null, "isObjectLike");
function isObject(input) {
  if (!isObjectLike(input) || Object.prototype.toString.call(input) !== "[object Object]") {
    return false;
  }
  if (Object.getPrototypeOf(input) === null) {
    return true;
  }
  let proto = input;
  while (Object.getPrototypeOf(proto) !== null) {
    proto = Object.getPrototypeOf(proto);
  }
  return Object.getPrototypeOf(input) === proto;
}
__name(isObject, "isObject");

// node_modules/jose/dist/webapi/lib/check_key_length.js
function checkKeyLength(alg, key) {
  if (alg.startsWith("RS") || alg.startsWith("PS")) {
    const { modulusLength } = key.algorithm;
    if (typeof modulusLength !== "number" || modulusLength < 2048) {
      throw new TypeError(`${alg} requires key modulusLength to be 2048 bits or larger`);
    }
  }
}
__name(checkKeyLength, "checkKeyLength");

// node_modules/jose/dist/webapi/lib/jwk_to_key.js
function subtleMapping(jwk) {
  let algorithm;
  let keyUsages;
  switch (jwk.kty) {
    case "AKP": {
      switch (jwk.alg) {
        case "ML-DSA-44":
        case "ML-DSA-65":
        case "ML-DSA-87":
          algorithm = { name: jwk.alg };
          keyUsages = jwk.priv ? ["sign"] : ["verify"];
          break;
        default:
          throw new JOSENotSupported('Invalid or unsupported JWK "alg" (Algorithm) Parameter value');
      }
      break;
    }
    case "RSA": {
      switch (jwk.alg) {
        case "PS256":
        case "PS384":
        case "PS512":
          algorithm = { name: "RSA-PSS", hash: `SHA-${jwk.alg.slice(-3)}` };
          keyUsages = jwk.d ? ["sign"] : ["verify"];
          break;
        case "RS256":
        case "RS384":
        case "RS512":
          algorithm = { name: "RSASSA-PKCS1-v1_5", hash: `SHA-${jwk.alg.slice(-3)}` };
          keyUsages = jwk.d ? ["sign"] : ["verify"];
          break;
        case "RSA-OAEP":
        case "RSA-OAEP-256":
        case "RSA-OAEP-384":
        case "RSA-OAEP-512":
          algorithm = {
            name: "RSA-OAEP",
            hash: `SHA-${parseInt(jwk.alg.slice(-3), 10) || 1}`
          };
          keyUsages = jwk.d ? ["decrypt", "unwrapKey"] : ["encrypt", "wrapKey"];
          break;
        default:
          throw new JOSENotSupported('Invalid or unsupported JWK "alg" (Algorithm) Parameter value');
      }
      break;
    }
    case "EC": {
      switch (jwk.alg) {
        case "ES256":
          algorithm = { name: "ECDSA", namedCurve: "P-256" };
          keyUsages = jwk.d ? ["sign"] : ["verify"];
          break;
        case "ES384":
          algorithm = { name: "ECDSA", namedCurve: "P-384" };
          keyUsages = jwk.d ? ["sign"] : ["verify"];
          break;
        case "ES512":
          algorithm = { name: "ECDSA", namedCurve: "P-521" };
          keyUsages = jwk.d ? ["sign"] : ["verify"];
          break;
        case "ECDH-ES":
        case "ECDH-ES+A128KW":
        case "ECDH-ES+A192KW":
        case "ECDH-ES+A256KW":
          algorithm = { name: "ECDH", namedCurve: jwk.crv };
          keyUsages = jwk.d ? ["deriveBits"] : [];
          break;
        default:
          throw new JOSENotSupported('Invalid or unsupported JWK "alg" (Algorithm) Parameter value');
      }
      break;
    }
    case "OKP": {
      switch (jwk.alg) {
        case "Ed25519":
        case "EdDSA":
          algorithm = { name: "Ed25519" };
          keyUsages = jwk.d ? ["sign"] : ["verify"];
          break;
        case "ECDH-ES":
        case "ECDH-ES+A128KW":
        case "ECDH-ES+A192KW":
        case "ECDH-ES+A256KW":
          algorithm = { name: jwk.crv };
          keyUsages = jwk.d ? ["deriveBits"] : [];
          break;
        default:
          throw new JOSENotSupported('Invalid or unsupported JWK "alg" (Algorithm) Parameter value');
      }
      break;
    }
    default:
      throw new JOSENotSupported('Invalid or unsupported JWK "kty" (Key Type) Parameter value');
  }
  return { algorithm, keyUsages };
}
__name(subtleMapping, "subtleMapping");
async function jwkToKey(jwk) {
  if (!jwk.alg) {
    throw new TypeError('"alg" argument is required when "jwk.alg" is not present');
  }
  const { algorithm, keyUsages } = subtleMapping(jwk);
  const keyData = { ...jwk };
  if (keyData.kty !== "AKP") {
    delete keyData.alg;
  }
  delete keyData.use;
  return crypto.subtle.importKey("jwk", keyData, algorithm, jwk.ext ?? (jwk.d || jwk.priv ? false : true), jwk.key_ops ?? keyUsages);
}
__name(jwkToKey, "jwkToKey");

// node_modules/jose/dist/webapi/lib/validate_crit.js
function validateCrit(Err, recognizedDefault, recognizedOption, protectedHeader, joseHeader) {
  if (joseHeader.crit !== void 0 && protectedHeader?.crit === void 0) {
    throw new Err('"crit" (Critical) Header Parameter MUST be integrity protected');
  }
  if (!protectedHeader || protectedHeader.crit === void 0) {
    return /* @__PURE__ */ new Set();
  }
  if (!Array.isArray(protectedHeader.crit) || protectedHeader.crit.length === 0 || protectedHeader.crit.some((input) => typeof input !== "string" || input.length === 0)) {
    throw new Err('"crit" (Critical) Header Parameter MUST be an array of non-empty strings when present');
  }
  let recognized;
  if (recognizedOption !== void 0) {
    recognized = new Map([...Object.entries(recognizedOption), ...recognizedDefault.entries()]);
  } else {
    recognized = recognizedDefault;
  }
  for (const parameter of protectedHeader.crit) {
    if (!recognized.has(parameter)) {
      throw new JOSENotSupported(`Extension Header Parameter "${parameter}" is not recognized`);
    }
    if (joseHeader[parameter] === void 0) {
      throw new Err(`Extension Header Parameter "${parameter}" is missing`);
    }
    if (recognized.get(parameter) && protectedHeader[parameter] === void 0) {
      throw new Err(`Extension Header Parameter "${parameter}" MUST be integrity protected`);
    }
  }
  return new Set(protectedHeader.crit);
}
__name(validateCrit, "validateCrit");

// node_modules/jose/dist/webapi/lib/validate_algorithms.js
function validateAlgorithms(option, algorithms) {
  if (algorithms !== void 0 && (!Array.isArray(algorithms) || algorithms.some((s) => typeof s !== "string"))) {
    throw new TypeError(`"${option}" option must be an array of strings`);
  }
  if (!algorithms) {
    return void 0;
  }
  return new Set(algorithms);
}
__name(validateAlgorithms, "validateAlgorithms");

// node_modules/jose/dist/webapi/lib/is_jwk.js
var isJWK = /* @__PURE__ */ __name((key) => isObject(key) && typeof key.kty === "string", "isJWK");
var isPrivateJWK = /* @__PURE__ */ __name((key) => key.kty !== "oct" && (key.kty === "AKP" && typeof key.priv === "string" || typeof key.d === "string"), "isPrivateJWK");
var isPublicJWK = /* @__PURE__ */ __name((key) => key.kty !== "oct" && key.d === void 0 && key.priv === void 0, "isPublicJWK");
var isSecretJWK = /* @__PURE__ */ __name((key) => key.kty === "oct" && typeof key.k === "string", "isSecretJWK");

// node_modules/jose/dist/webapi/lib/normalize_key.js
var cache;
var handleJWK = /* @__PURE__ */ __name(async (key, jwk, alg, freeze = false) => {
  cache ||= /* @__PURE__ */ new WeakMap();
  let cached = cache.get(key);
  if (cached?.[alg]) {
    return cached[alg];
  }
  const cryptoKey = await jwkToKey({ ...jwk, alg });
  if (freeze)
    Object.freeze(key);
  if (!cached) {
    cache.set(key, { [alg]: cryptoKey });
  } else {
    cached[alg] = cryptoKey;
  }
  return cryptoKey;
}, "handleJWK");
var handleKeyObject = /* @__PURE__ */ __name((keyObject, alg) => {
  cache ||= /* @__PURE__ */ new WeakMap();
  let cached = cache.get(keyObject);
  if (cached?.[alg]) {
    return cached[alg];
  }
  const isPublic = keyObject.type === "public";
  const extractable = isPublic ? true : false;
  let cryptoKey;
  if (keyObject.asymmetricKeyType === "x25519") {
    switch (alg) {
      case "ECDH-ES":
      case "ECDH-ES+A128KW":
      case "ECDH-ES+A192KW":
      case "ECDH-ES+A256KW":
        break;
      default:
        throw new TypeError("given KeyObject instance cannot be used for this algorithm");
    }
    cryptoKey = keyObject.toCryptoKey(keyObject.asymmetricKeyType, extractable, isPublic ? [] : ["deriveBits"]);
  }
  if (keyObject.asymmetricKeyType === "ed25519") {
    if (alg !== "EdDSA" && alg !== "Ed25519") {
      throw new TypeError("given KeyObject instance cannot be used for this algorithm");
    }
    cryptoKey = keyObject.toCryptoKey(keyObject.asymmetricKeyType, extractable, [
      isPublic ? "verify" : "sign"
    ]);
  }
  switch (keyObject.asymmetricKeyType) {
    case "ml-dsa-44":
    case "ml-dsa-65":
    case "ml-dsa-87": {
      if (alg !== keyObject.asymmetricKeyType.toUpperCase()) {
        throw new TypeError("given KeyObject instance cannot be used for this algorithm");
      }
      cryptoKey = keyObject.toCryptoKey(keyObject.asymmetricKeyType, extractable, [
        isPublic ? "verify" : "sign"
      ]);
    }
  }
  if (keyObject.asymmetricKeyType === "rsa") {
    let hash;
    switch (alg) {
      case "RSA-OAEP":
        hash = "SHA-1";
        break;
      case "RS256":
      case "PS256":
      case "RSA-OAEP-256":
        hash = "SHA-256";
        break;
      case "RS384":
      case "PS384":
      case "RSA-OAEP-384":
        hash = "SHA-384";
        break;
      case "RS512":
      case "PS512":
      case "RSA-OAEP-512":
        hash = "SHA-512";
        break;
      default:
        throw new TypeError("given KeyObject instance cannot be used for this algorithm");
    }
    if (alg.startsWith("RSA-OAEP")) {
      return keyObject.toCryptoKey({
        name: "RSA-OAEP",
        hash
      }, extractable, isPublic ? ["encrypt"] : ["decrypt"]);
    }
    cryptoKey = keyObject.toCryptoKey({
      name: alg.startsWith("PS") ? "RSA-PSS" : "RSASSA-PKCS1-v1_5",
      hash
    }, extractable, [isPublic ? "verify" : "sign"]);
  }
  if (keyObject.asymmetricKeyType === "ec") {
    const nist = /* @__PURE__ */ new Map([
      ["prime256v1", "P-256"],
      ["secp384r1", "P-384"],
      ["secp521r1", "P-521"]
    ]);
    const namedCurve = nist.get(keyObject.asymmetricKeyDetails?.namedCurve);
    if (!namedCurve) {
      throw new TypeError("given KeyObject instance cannot be used for this algorithm");
    }
    if (alg === "ES256" && namedCurve === "P-256") {
      cryptoKey = keyObject.toCryptoKey({
        name: "ECDSA",
        namedCurve
      }, extractable, [isPublic ? "verify" : "sign"]);
    }
    if (alg === "ES384" && namedCurve === "P-384") {
      cryptoKey = keyObject.toCryptoKey({
        name: "ECDSA",
        namedCurve
      }, extractable, [isPublic ? "verify" : "sign"]);
    }
    if (alg === "ES512" && namedCurve === "P-521") {
      cryptoKey = keyObject.toCryptoKey({
        name: "ECDSA",
        namedCurve
      }, extractable, [isPublic ? "verify" : "sign"]);
    }
    if (alg.startsWith("ECDH-ES")) {
      cryptoKey = keyObject.toCryptoKey({
        name: "ECDH",
        namedCurve
      }, extractable, isPublic ? [] : ["deriveBits"]);
    }
  }
  if (!cryptoKey) {
    throw new TypeError("given KeyObject instance cannot be used for this algorithm");
  }
  if (!cached) {
    cache.set(keyObject, { [alg]: cryptoKey });
  } else {
    cached[alg] = cryptoKey;
  }
  return cryptoKey;
}, "handleKeyObject");
async function normalizeKey(key, alg) {
  if (key instanceof Uint8Array) {
    return key;
  }
  if (isCryptoKey(key)) {
    return key;
  }
  if (isKeyObject(key)) {
    if (key.type === "secret") {
      return key.export();
    }
    if ("toCryptoKey" in key && typeof key.toCryptoKey === "function") {
      try {
        return handleKeyObject(key, alg);
      } catch (err) {
        if (err instanceof TypeError) {
          throw err;
        }
      }
    }
    let jwk = key.export({ format: "jwk" });
    return handleJWK(key, jwk, alg);
  }
  if (isJWK(key)) {
    if (key.k) {
      return decode(key.k);
    }
    return handleJWK(key, key, alg, true);
  }
  throw new Error("unreachable");
}
__name(normalizeKey, "normalizeKey");

// node_modules/jose/dist/webapi/lib/check_key_type.js
var tag = /* @__PURE__ */ __name((key) => key?.[Symbol.toStringTag], "tag");
var jwkMatchesOp = /* @__PURE__ */ __name((alg, key, usage) => {
  if (key.use !== void 0) {
    let expected;
    switch (usage) {
      case "sign":
      case "verify":
        expected = "sig";
        break;
      case "encrypt":
      case "decrypt":
        expected = "enc";
        break;
    }
    if (key.use !== expected) {
      throw new TypeError(`Invalid key for this operation, its "use" must be "${expected}" when present`);
    }
  }
  if (key.alg !== void 0 && key.alg !== alg) {
    throw new TypeError(`Invalid key for this operation, its "alg" must be "${alg}" when present`);
  }
  if (Array.isArray(key.key_ops)) {
    let expectedKeyOp;
    switch (true) {
      case (usage === "sign" || usage === "verify"):
      case alg === "dir":
      case alg.includes("CBC-HS"):
        expectedKeyOp = usage;
        break;
      case alg.startsWith("PBES2"):
        expectedKeyOp = "deriveBits";
        break;
      case /^A\d{3}(?:GCM)?(?:KW)?$/.test(alg):
        if (!alg.includes("GCM") && alg.endsWith("KW")) {
          expectedKeyOp = usage === "encrypt" ? "wrapKey" : "unwrapKey";
        } else {
          expectedKeyOp = usage;
        }
        break;
      case (usage === "encrypt" && alg.startsWith("RSA")):
        expectedKeyOp = "wrapKey";
        break;
      case usage === "decrypt":
        expectedKeyOp = alg.startsWith("RSA") ? "unwrapKey" : "deriveBits";
        break;
    }
    if (expectedKeyOp && key.key_ops?.includes?.(expectedKeyOp) === false) {
      throw new TypeError(`Invalid key for this operation, its "key_ops" must include "${expectedKeyOp}" when present`);
    }
  }
  return true;
}, "jwkMatchesOp");
var symmetricTypeCheck = /* @__PURE__ */ __name((alg, key, usage) => {
  if (key instanceof Uint8Array)
    return;
  if (isJWK(key)) {
    if (isSecretJWK(key) && jwkMatchesOp(alg, key, usage))
      return;
    throw new TypeError(`JSON Web Key for symmetric algorithms must have JWK "kty" (Key Type) equal to "oct" and the JWK "k" (Key Value) present`);
  }
  if (!isKeyLike(key)) {
    throw new TypeError(withAlg(alg, key, "CryptoKey", "KeyObject", "JSON Web Key", "Uint8Array"));
  }
  if (key.type !== "secret") {
    throw new TypeError(`${tag(key)} instances for symmetric algorithms must be of type "secret"`);
  }
}, "symmetricTypeCheck");
var asymmetricTypeCheck = /* @__PURE__ */ __name((alg, key, usage) => {
  if (isJWK(key)) {
    switch (usage) {
      case "decrypt":
      case "sign":
        if (isPrivateJWK(key) && jwkMatchesOp(alg, key, usage))
          return;
        throw new TypeError(`JSON Web Key for this operation must be a private JWK`);
      case "encrypt":
      case "verify":
        if (isPublicJWK(key) && jwkMatchesOp(alg, key, usage))
          return;
        throw new TypeError(`JSON Web Key for this operation must be a public JWK`);
    }
  }
  if (!isKeyLike(key)) {
    throw new TypeError(withAlg(alg, key, "CryptoKey", "KeyObject", "JSON Web Key"));
  }
  if (key.type === "secret") {
    throw new TypeError(`${tag(key)} instances for asymmetric algorithms must not be of type "secret"`);
  }
  if (key.type === "public") {
    switch (usage) {
      case "sign":
        throw new TypeError(`${tag(key)} instances for asymmetric algorithm signing must be of type "private"`);
      case "decrypt":
        throw new TypeError(`${tag(key)} instances for asymmetric algorithm decryption must be of type "private"`);
    }
  }
  if (key.type === "private") {
    switch (usage) {
      case "verify":
        throw new TypeError(`${tag(key)} instances for asymmetric algorithm verifying must be of type "public"`);
      case "encrypt":
        throw new TypeError(`${tag(key)} instances for asymmetric algorithm encryption must be of type "public"`);
    }
  }
}, "asymmetricTypeCheck");
function checkKeyType(alg, key, usage) {
  switch (alg.substring(0, 2)) {
    case "A1":
    case "A2":
    case "di":
    case "HS":
    case "PB":
      symmetricTypeCheck(alg, key, usage);
      break;
    default:
      asymmetricTypeCheck(alg, key, usage);
  }
}
__name(checkKeyType, "checkKeyType");

// node_modules/jose/dist/webapi/lib/subtle_dsa.js
function subtleAlgorithm(alg, algorithm) {
  const hash = `SHA-${alg.slice(-3)}`;
  switch (alg) {
    case "HS256":
    case "HS384":
    case "HS512":
      return { hash, name: "HMAC" };
    case "PS256":
    case "PS384":
    case "PS512":
      return { hash, name: "RSA-PSS", saltLength: parseInt(alg.slice(-3), 10) >> 3 };
    case "RS256":
    case "RS384":
    case "RS512":
      return { hash, name: "RSASSA-PKCS1-v1_5" };
    case "ES256":
    case "ES384":
    case "ES512":
      return { hash, name: "ECDSA", namedCurve: algorithm.namedCurve };
    case "Ed25519":
    case "EdDSA":
      return { name: "Ed25519" };
    case "ML-DSA-44":
    case "ML-DSA-65":
    case "ML-DSA-87":
      return { name: alg };
    default:
      throw new JOSENotSupported(`alg ${alg} is not supported either by JOSE or your javascript runtime`);
  }
}
__name(subtleAlgorithm, "subtleAlgorithm");

// node_modules/jose/dist/webapi/lib/get_sign_verify_key.js
async function getSigKey(alg, key, usage) {
  if (key instanceof Uint8Array) {
    if (!alg.startsWith("HS")) {
      throw new TypeError(invalidKeyInput(key, "CryptoKey", "KeyObject", "JSON Web Key"));
    }
    return crypto.subtle.importKey("raw", key, { hash: `SHA-${alg.slice(-3)}`, name: "HMAC" }, false, [usage]);
  }
  checkSigCryptoKey(key, alg, usage);
  return key;
}
__name(getSigKey, "getSigKey");

// node_modules/jose/dist/webapi/lib/verify.js
async function verify(alg, key, signature, data) {
  const cryptoKey = await getSigKey(alg, key, "verify");
  checkKeyLength(alg, cryptoKey);
  const algorithm = subtleAlgorithm(alg, cryptoKey.algorithm);
  try {
    return await crypto.subtle.verify(algorithm, cryptoKey, signature, data);
  } catch {
    return false;
  }
}
__name(verify, "verify");

// node_modules/jose/dist/webapi/jws/flattened/verify.js
async function flattenedVerify(jws, key, options) {
  if (!isObject(jws)) {
    throw new JWSInvalid("Flattened JWS must be an object");
  }
  if (jws.protected === void 0 && jws.header === void 0) {
    throw new JWSInvalid('Flattened JWS must have either of the "protected" or "header" members');
  }
  if (jws.protected !== void 0 && typeof jws.protected !== "string") {
    throw new JWSInvalid("JWS Protected Header incorrect type");
  }
  if (jws.payload === void 0) {
    throw new JWSInvalid("JWS Payload missing");
  }
  if (typeof jws.signature !== "string") {
    throw new JWSInvalid("JWS Signature missing or incorrect type");
  }
  if (jws.header !== void 0 && !isObject(jws.header)) {
    throw new JWSInvalid("JWS Unprotected Header incorrect type");
  }
  let parsedProt = {};
  if (jws.protected) {
    try {
      const protectedHeader = decode(jws.protected);
      parsedProt = JSON.parse(decoder.decode(protectedHeader));
    } catch {
      throw new JWSInvalid("JWS Protected Header is invalid");
    }
  }
  if (!isDisjoint(parsedProt, jws.header)) {
    throw new JWSInvalid("JWS Protected and JWS Unprotected Header Parameter names must be disjoint");
  }
  const joseHeader = {
    ...parsedProt,
    ...jws.header
  };
  const extensions = validateCrit(JWSInvalid, /* @__PURE__ */ new Map([["b64", true]]), options?.crit, parsedProt, joseHeader);
  let b64 = true;
  if (extensions.has("b64")) {
    b64 = parsedProt.b64;
    if (typeof b64 !== "boolean") {
      throw new JWSInvalid('The "b64" (base64url-encode payload) Header Parameter must be a boolean');
    }
  }
  const { alg } = joseHeader;
  if (typeof alg !== "string" || !alg) {
    throw new JWSInvalid('JWS "alg" (Algorithm) Header Parameter missing or invalid');
  }
  const algorithms = options && validateAlgorithms("algorithms", options.algorithms);
  if (algorithms && !algorithms.has(alg)) {
    throw new JOSEAlgNotAllowed('"alg" (Algorithm) Header Parameter value not allowed');
  }
  if (b64) {
    if (typeof jws.payload !== "string") {
      throw new JWSInvalid("JWS Payload must be a string");
    }
  } else if (typeof jws.payload !== "string" && !(jws.payload instanceof Uint8Array)) {
    throw new JWSInvalid("JWS Payload must be a string or an Uint8Array instance");
  }
  let resolvedKey = false;
  if (typeof key === "function") {
    key = await key(parsedProt, jws);
    resolvedKey = true;
  }
  checkKeyType(alg, key, "verify");
  const data = concat(jws.protected !== void 0 ? encode(jws.protected) : new Uint8Array(), encode("."), typeof jws.payload === "string" ? b64 ? encode(jws.payload) : encoder.encode(jws.payload) : jws.payload);
  let signature;
  try {
    signature = decode(jws.signature);
  } catch {
    throw new JWSInvalid("Failed to base64url decode the signature");
  }
  const k = await normalizeKey(key, alg);
  const verified = await verify(alg, k, signature, data);
  if (!verified) {
    throw new JWSSignatureVerificationFailed();
  }
  let payload;
  if (b64) {
    try {
      payload = decode(jws.payload);
    } catch {
      throw new JWSInvalid("Failed to base64url decode the payload");
    }
  } else if (typeof jws.payload === "string") {
    payload = encoder.encode(jws.payload);
  } else {
    payload = jws.payload;
  }
  const result = { payload };
  if (jws.protected !== void 0) {
    result.protectedHeader = parsedProt;
  }
  if (jws.header !== void 0) {
    result.unprotectedHeader = jws.header;
  }
  if (resolvedKey) {
    return { ...result, key: k };
  }
  return result;
}
__name(flattenedVerify, "flattenedVerify");

// node_modules/jose/dist/webapi/jws/compact/verify.js
async function compactVerify(jws, key, options) {
  if (jws instanceof Uint8Array) {
    jws = decoder.decode(jws);
  }
  if (typeof jws !== "string") {
    throw new JWSInvalid("Compact JWS must be a string or Uint8Array");
  }
  const { 0: protectedHeader, 1: payload, 2: signature, length } = jws.split(".");
  if (length !== 3) {
    throw new JWSInvalid("Invalid Compact JWS");
  }
  const verified = await flattenedVerify({ payload, protected: protectedHeader, signature }, key, options);
  const result = { payload: verified.payload, protectedHeader: verified.protectedHeader };
  if (typeof key === "function") {
    return { ...result, key: verified.key };
  }
  return result;
}
__name(compactVerify, "compactVerify");

// node_modules/jose/dist/webapi/lib/jwt_claims_set.js
var epoch = /* @__PURE__ */ __name((date) => Math.floor(date.getTime() / 1e3), "epoch");
var minute = 60;
var hour = minute * 60;
var day = hour * 24;
var week = day * 7;
var year = day * 365.25;
var REGEX = /^(\+|\-)? ?(\d+|\d+\.\d+) ?(seconds?|secs?|s|minutes?|mins?|m|hours?|hrs?|h|days?|d|weeks?|w|years?|yrs?|y)(?: (ago|from now))?$/i;
function secs(str) {
  const matched = REGEX.exec(str);
  if (!matched || matched[4] && matched[1]) {
    throw new TypeError("Invalid time period format");
  }
  const value = parseFloat(matched[2]);
  const unit = matched[3].toLowerCase();
  let numericDate;
  switch (unit) {
    case "sec":
    case "secs":
    case "second":
    case "seconds":
    case "s":
      numericDate = Math.round(value);
      break;
    case "minute":
    case "minutes":
    case "min":
    case "mins":
    case "m":
      numericDate = Math.round(value * minute);
      break;
    case "hour":
    case "hours":
    case "hr":
    case "hrs":
    case "h":
      numericDate = Math.round(value * hour);
      break;
    case "day":
    case "days":
    case "d":
      numericDate = Math.round(value * day);
      break;
    case "week":
    case "weeks":
    case "w":
      numericDate = Math.round(value * week);
      break;
    default:
      numericDate = Math.round(value * year);
      break;
  }
  if (matched[1] === "-" || matched[4] === "ago") {
    return -numericDate;
  }
  return numericDate;
}
__name(secs, "secs");
function validateInput(label, input) {
  if (!Number.isFinite(input)) {
    throw new TypeError(`Invalid ${label} input`);
  }
  return input;
}
__name(validateInput, "validateInput");
var normalizeTyp = /* @__PURE__ */ __name((value) => {
  if (value.includes("/")) {
    return value.toLowerCase();
  }
  return `application/${value.toLowerCase()}`;
}, "normalizeTyp");
var checkAudiencePresence = /* @__PURE__ */ __name((audPayload, audOption) => {
  if (typeof audPayload === "string") {
    return audOption.includes(audPayload);
  }
  if (Array.isArray(audPayload)) {
    return audOption.some(Set.prototype.has.bind(new Set(audPayload)));
  }
  return false;
}, "checkAudiencePresence");
function validateClaimsSet(protectedHeader, encodedPayload, options = {}) {
  let payload;
  try {
    payload = JSON.parse(decoder.decode(encodedPayload));
  } catch {
  }
  if (!isObject(payload)) {
    throw new JWTInvalid("JWT Claims Set must be a top-level JSON object");
  }
  const { typ } = options;
  if (typ && (typeof protectedHeader.typ !== "string" || normalizeTyp(protectedHeader.typ) !== normalizeTyp(typ))) {
    throw new JWTClaimValidationFailed('unexpected "typ" JWT header value', payload, "typ", "check_failed");
  }
  const { requiredClaims = [], issuer, subject, audience, maxTokenAge } = options;
  const presenceCheck = [...requiredClaims];
  if (maxTokenAge !== void 0)
    presenceCheck.push("iat");
  if (audience !== void 0)
    presenceCheck.push("aud");
  if (subject !== void 0)
    presenceCheck.push("sub");
  if (issuer !== void 0)
    presenceCheck.push("iss");
  for (const claim of new Set(presenceCheck.reverse())) {
    if (!(claim in payload)) {
      throw new JWTClaimValidationFailed(`missing required "${claim}" claim`, payload, claim, "missing");
    }
  }
  if (issuer && !(Array.isArray(issuer) ? issuer : [issuer]).includes(payload.iss)) {
    throw new JWTClaimValidationFailed('unexpected "iss" claim value', payload, "iss", "check_failed");
  }
  if (subject && payload.sub !== subject) {
    throw new JWTClaimValidationFailed('unexpected "sub" claim value', payload, "sub", "check_failed");
  }
  if (audience && !checkAudiencePresence(payload.aud, typeof audience === "string" ? [audience] : audience)) {
    throw new JWTClaimValidationFailed('unexpected "aud" claim value', payload, "aud", "check_failed");
  }
  let tolerance;
  switch (typeof options.clockTolerance) {
    case "string":
      tolerance = secs(options.clockTolerance);
      break;
    case "number":
      tolerance = options.clockTolerance;
      break;
    case "undefined":
      tolerance = 0;
      break;
    default:
      throw new TypeError("Invalid clockTolerance option type");
  }
  const { currentDate } = options;
  const now = epoch(currentDate || /* @__PURE__ */ new Date());
  if ((payload.iat !== void 0 || maxTokenAge) && typeof payload.iat !== "number") {
    throw new JWTClaimValidationFailed('"iat" claim must be a number', payload, "iat", "invalid");
  }
  if (payload.nbf !== void 0) {
    if (typeof payload.nbf !== "number") {
      throw new JWTClaimValidationFailed('"nbf" claim must be a number', payload, "nbf", "invalid");
    }
    if (payload.nbf > now + tolerance) {
      throw new JWTClaimValidationFailed('"nbf" claim timestamp check failed', payload, "nbf", "check_failed");
    }
  }
  if (payload.exp !== void 0) {
    if (typeof payload.exp !== "number") {
      throw new JWTClaimValidationFailed('"exp" claim must be a number', payload, "exp", "invalid");
    }
    if (payload.exp <= now - tolerance) {
      throw new JWTExpired('"exp" claim timestamp check failed', payload, "exp", "check_failed");
    }
  }
  if (maxTokenAge) {
    const age = now - payload.iat;
    const max = typeof maxTokenAge === "number" ? maxTokenAge : secs(maxTokenAge);
    if (age - tolerance > max) {
      throw new JWTExpired('"iat" claim timestamp check failed (too far in the past)', payload, "iat", "check_failed");
    }
    if (age < 0 - tolerance) {
      throw new JWTClaimValidationFailed('"iat" claim timestamp check failed (it should be in the past)', payload, "iat", "check_failed");
    }
  }
  return payload;
}
__name(validateClaimsSet, "validateClaimsSet");
var JWTClaimsBuilder = class {
  #payload;
  constructor(payload) {
    if (!isObject(payload)) {
      throw new TypeError("JWT Claims Set MUST be an object");
    }
    this.#payload = structuredClone(payload);
  }
  data() {
    return encoder.encode(JSON.stringify(this.#payload));
  }
  get iss() {
    return this.#payload.iss;
  }
  set iss(value) {
    this.#payload.iss = value;
  }
  get sub() {
    return this.#payload.sub;
  }
  set sub(value) {
    this.#payload.sub = value;
  }
  get aud() {
    return this.#payload.aud;
  }
  set aud(value) {
    this.#payload.aud = value;
  }
  set jti(value) {
    this.#payload.jti = value;
  }
  set nbf(value) {
    if (typeof value === "number") {
      this.#payload.nbf = validateInput("setNotBefore", value);
    } else if (value instanceof Date) {
      this.#payload.nbf = validateInput("setNotBefore", epoch(value));
    } else {
      this.#payload.nbf = epoch(/* @__PURE__ */ new Date()) + secs(value);
    }
  }
  set exp(value) {
    if (typeof value === "number") {
      this.#payload.exp = validateInput("setExpirationTime", value);
    } else if (value instanceof Date) {
      this.#payload.exp = validateInput("setExpirationTime", epoch(value));
    } else {
      this.#payload.exp = epoch(/* @__PURE__ */ new Date()) + secs(value);
    }
  }
  set iat(value) {
    if (value === void 0) {
      this.#payload.iat = epoch(/* @__PURE__ */ new Date());
    } else if (value instanceof Date) {
      this.#payload.iat = validateInput("setIssuedAt", epoch(value));
    } else if (typeof value === "string") {
      this.#payload.iat = validateInput("setIssuedAt", epoch(/* @__PURE__ */ new Date()) + secs(value));
    } else {
      this.#payload.iat = validateInput("setIssuedAt", value);
    }
  }
};
__name(JWTClaimsBuilder, "JWTClaimsBuilder");

// node_modules/jose/dist/webapi/jwt/verify.js
async function jwtVerify(jwt, key, options) {
  const verified = await compactVerify(jwt, key, options);
  if (verified.protectedHeader.crit?.includes("b64") && verified.protectedHeader.b64 === false) {
    throw new JWTInvalid("JWTs MUST NOT use unencoded payload");
  }
  const payload = validateClaimsSet(verified.protectedHeader, verified.payload, options);
  const result = { payload, protectedHeader: verified.protectedHeader };
  if (typeof key === "function") {
    return { ...result, key: verified.key };
  }
  return result;
}
__name(jwtVerify, "jwtVerify");

// node_modules/jose/dist/webapi/lib/sign.js
async function sign(alg, key, data) {
  const cryptoKey = await getSigKey(alg, key, "sign");
  checkKeyLength(alg, cryptoKey);
  const signature = await crypto.subtle.sign(subtleAlgorithm(alg, cryptoKey.algorithm), cryptoKey, data);
  return new Uint8Array(signature);
}
__name(sign, "sign");

// node_modules/jose/dist/webapi/jws/flattened/sign.js
var FlattenedSign = class {
  #payload;
  #protectedHeader;
  #unprotectedHeader;
  constructor(payload) {
    if (!(payload instanceof Uint8Array)) {
      throw new TypeError("payload must be an instance of Uint8Array");
    }
    this.#payload = payload;
  }
  setProtectedHeader(protectedHeader) {
    if (this.#protectedHeader) {
      throw new TypeError("setProtectedHeader can only be called once");
    }
    this.#protectedHeader = protectedHeader;
    return this;
  }
  setUnprotectedHeader(unprotectedHeader) {
    if (this.#unprotectedHeader) {
      throw new TypeError("setUnprotectedHeader can only be called once");
    }
    this.#unprotectedHeader = unprotectedHeader;
    return this;
  }
  async sign(key, options) {
    if (!this.#protectedHeader && !this.#unprotectedHeader) {
      throw new JWSInvalid("either setProtectedHeader or setUnprotectedHeader must be called before #sign()");
    }
    if (!isDisjoint(this.#protectedHeader, this.#unprotectedHeader)) {
      throw new JWSInvalid("JWS Protected and JWS Unprotected Header Parameter names must be disjoint");
    }
    const joseHeader = {
      ...this.#protectedHeader,
      ...this.#unprotectedHeader
    };
    const extensions = validateCrit(JWSInvalid, /* @__PURE__ */ new Map([["b64", true]]), options?.crit, this.#protectedHeader, joseHeader);
    let b64 = true;
    if (extensions.has("b64")) {
      b64 = this.#protectedHeader.b64;
      if (typeof b64 !== "boolean") {
        throw new JWSInvalid('The "b64" (base64url-encode payload) Header Parameter must be a boolean');
      }
    }
    const { alg } = joseHeader;
    if (typeof alg !== "string" || !alg) {
      throw new JWSInvalid('JWS "alg" (Algorithm) Header Parameter missing or invalid');
    }
    checkKeyType(alg, key, "sign");
    let payloadS;
    let payloadB;
    if (b64) {
      payloadS = encode2(this.#payload);
      payloadB = encode(payloadS);
    } else {
      payloadB = this.#payload;
      payloadS = "";
    }
    let protectedHeaderString;
    let protectedHeaderBytes;
    if (this.#protectedHeader) {
      protectedHeaderString = encode2(JSON.stringify(this.#protectedHeader));
      protectedHeaderBytes = encode(protectedHeaderString);
    } else {
      protectedHeaderString = "";
      protectedHeaderBytes = new Uint8Array();
    }
    const data = concat(protectedHeaderBytes, encode("."), payloadB);
    const k = await normalizeKey(key, alg);
    const signature = await sign(alg, k, data);
    const jws = {
      signature: encode2(signature),
      payload: payloadS
    };
    if (this.#unprotectedHeader) {
      jws.header = this.#unprotectedHeader;
    }
    if (this.#protectedHeader) {
      jws.protected = protectedHeaderString;
    }
    return jws;
  }
};
__name(FlattenedSign, "FlattenedSign");

// node_modules/jose/dist/webapi/jws/compact/sign.js
var CompactSign = class {
  #flattened;
  constructor(payload) {
    this.#flattened = new FlattenedSign(payload);
  }
  setProtectedHeader(protectedHeader) {
    this.#flattened.setProtectedHeader(protectedHeader);
    return this;
  }
  async sign(key, options) {
    const jws = await this.#flattened.sign(key, options);
    if (jws.payload === void 0) {
      throw new TypeError("use the flattened module for creating JWS with b64: false");
    }
    return `${jws.protected}.${jws.payload}.${jws.signature}`;
  }
};
__name(CompactSign, "CompactSign");

// node_modules/jose/dist/webapi/jwt/sign.js
var SignJWT = class {
  #protectedHeader;
  #jwt;
  constructor(payload = {}) {
    this.#jwt = new JWTClaimsBuilder(payload);
  }
  setIssuer(issuer) {
    this.#jwt.iss = issuer;
    return this;
  }
  setSubject(subject) {
    this.#jwt.sub = subject;
    return this;
  }
  setAudience(audience) {
    this.#jwt.aud = audience;
    return this;
  }
  setJti(jwtId) {
    this.#jwt.jti = jwtId;
    return this;
  }
  setNotBefore(input) {
    this.#jwt.nbf = input;
    return this;
  }
  setExpirationTime(input) {
    this.#jwt.exp = input;
    return this;
  }
  setIssuedAt(input) {
    this.#jwt.iat = input;
    return this;
  }
  setProtectedHeader(protectedHeader) {
    this.#protectedHeader = protectedHeader;
    return this;
  }
  async sign(key, options) {
    const sig = new CompactSign(this.#jwt.data());
    sig.setProtectedHeader(this.#protectedHeader);
    if (Array.isArray(this.#protectedHeader?.crit) && this.#protectedHeader.crit.includes("b64") && this.#protectedHeader.b64 === false) {
      throw new JWTInvalid("JWTs MUST NOT use unencoded payload");
    }
    return sig.sign(key, options);
  }
};
__name(SignJWT, "SignJWT");

// src/utils/tokens.ts
var ACCESS_TOKEN_EXPIRY = "15m";
var REFRESH_TOKEN_EXPIRY = "7d";
function generateId3() {
  const array = new Uint8Array(16);
  crypto.getRandomValues(array);
  return Array.from(array, (byte) => byte.toString(16).padStart(2, "0")).join("");
}
__name(generateId3, "generateId");
async function createAccessToken(payload, secret) {
  const secretKey = new TextEncoder().encode(secret);
  return new SignJWT(payload).setProtectedHeader({ alg: "HS256" }).setIssuedAt().setExpirationTime(ACCESS_TOKEN_EXPIRY).sign(secretKey);
}
__name(createAccessToken, "createAccessToken");
async function createRefreshToken(userId, sessionId, secret) {
  const secretKey = new TextEncoder().encode(secret);
  return new SignJWT({ sub: userId, jti: sessionId }).setProtectedHeader({ alg: "HS256" }).setIssuedAt().setExpirationTime(REFRESH_TOKEN_EXPIRY).sign(secretKey);
}
__name(createRefreshToken, "createRefreshToken");
async function verifyAccessToken(token, secret) {
  try {
    const secretKey = new TextEncoder().encode(secret);
    const { payload } = await jwtVerify(token, secretKey);
    return payload;
  } catch {
    return null;
  }
}
__name(verifyAccessToken, "verifyAccessToken");
async function verifyRefreshToken(token, secret) {
  try {
    const secretKey = new TextEncoder().encode(secret);
    const { payload } = await jwtVerify(token, secretKey);
    return payload;
  } catch {
    return null;
  }
}
__name(verifyRefreshToken, "verifyRefreshToken");
async function hashRefreshToken(token) {
  const encoder2 = new TextEncoder();
  const data = encoder2.encode(token);
  const hashBuffer = await crypto.subtle.digest("SHA-256", data);
  const hashArray = Array.from(new Uint8Array(hashBuffer));
  return hashArray.map((b) => b.toString(16).padStart(2, "0")).join("");
}
__name(hashRefreshToken, "hashRefreshToken");
function getRefreshTokenExpiry() {
  return Date.now() + 7 * 24 * 60 * 60 * 1e3;
}
__name(getRefreshTokenExpiry, "getRefreshTokenExpiry");

// src/middleware/jwt.ts
async function jwtAuth(c, next) {
  const authHeader = c.req.header("Authorization");
  c.set("user", null);
  c.set("userId", null);
  c.set("userEmail", null);
  c.set("userTenants", []);
  c.set("authMethod", null);
  if (!authHeader) {
    return next();
  }
  if (authHeader.startsWith("Bearer ")) {
    const token = authHeader.slice(7);
    if (!token) {
      return c.json({ error: "Invalid authorization header" }, 401);
    }
    const secret = c.env.JWT_SECRET;
    if (!secret) {
      console.error("JWT_SECRET not configured");
      return c.json({ error: "Server configuration error" }, 500);
    }
    const payload = await verifyAccessToken(token, secret);
    if (!payload) {
      return c.json({ error: "Invalid or expired token" }, 401);
    }
    c.set("user", payload);
    c.set("userId", payload.sub);
    c.set("userEmail", payload.email);
    c.set("userTenants", payload.tenants || []);
    c.set("authMethod", "jwt");
  }
  return next();
}
__name(jwtAuth, "jwtAuth");
async function requireJwtAuth(c, next) {
  const user = c.get("user");
  const authMethod = c.get("authMethod");
  if (!user || authMethod !== "jwt") {
    return c.json({ error: "Authentication required" }, 401);
  }
  return next();
}
__name(requireJwtAuth, "requireJwtAuth");
function hasAccessToTenant(c, tenantId) {
  const tenants2 = c.get("userTenants") || [];
  return tenants2.some((t) => t.id === tenantId);
}
__name(hasAccessToTenant, "hasAccessToTenant");
function getUserRoleInTenant(c, tenantId) {
  const tenants2 = c.get("userTenants") || [];
  const tenant = tenants2.find((t) => t.id === tenantId);
  return tenant ? tenant.role : null;
}
__name(getUserRoleInTenant, "getUserRoleInTenant");

// src/middleware/tenant.ts
async function extractTenant(c, next) {
  c.set("tenantId", null);
  c.set("tenantName", null);
  c.set("tenantRole", null);
  c.set("tenantPlan", null);
  const tenantIdHeader = c.req.header("X-Tenant-Id");
  const authMethod = c.get("authMethod");
  if (authMethod === "jwt" && tenantIdHeader) {
    if (!hasAccessToTenant(c, tenantIdHeader)) {
      return c.json({ error: "Access denied to this tenant" }, 403);
    }
    const role = getUserRoleInTenant(c, tenantIdHeader);
    const db = c.env.DB;
    const tenant = await db.prepare("SELECT id, name, plan FROM tenants WHERE id = ?").bind(tenantIdHeader).first();
    if (!tenant) {
      return c.json({ error: "Tenant not found" }, 404);
    }
    c.set("tenantId", tenant.id);
    c.set("tenantName", tenant.name);
    c.set("tenantRole", role);
    c.set("tenantPlan", tenant.plan);
  } else if (authMethod === "apikey") {
    const apiKey = c.req.header("X-API-Key");
    if (apiKey) {
      const db = c.env.DB;
      const keyData = await db.prepare(`
          SELECT ak.tenant_id, t.name, t.plan
          FROM api_keys ak
          LEFT JOIN tenants t ON ak.tenant_id = t.id
          WHERE ak.key = ? AND ak.is_active = 1
        `).bind(apiKey).first();
      if (keyData?.tenant_id) {
        c.set("tenantId", keyData.tenant_id);
        c.set("tenantName", keyData.name);
        c.set("tenantRole", "admin");
        c.set("tenantPlan", keyData.plan);
      }
    }
  }
  return next();
}
__name(extractTenant, "extractTenant");
function requireTenantRole(minRoles) {
  return async (c, next) => {
    const tenantId = c.get("tenantId");
    const authMethod = c.get("authMethod");
    if (!tenantId) {
      return c.json({ error: "Tenant selection required" }, 400);
    }
    if (authMethod === "apikey") {
      return next();
    }
    const role = c.get("tenantRole");
    if (!role || !minRoles.includes(role)) {
      return c.json({
        error: "Insufficient permissions",
        required: minRoles,
        current: role
      }, 403);
    }
    return next();
  };
}
__name(requireTenantRole, "requireTenantRole");
var requireWriteAccess = requireTenantRole(["owner", "admin", "member"]);
var requireManageAccess = requireTenantRole(["owner", "admin"]);
var requireOwnerAccess = requireTenantRole(["owner"]);

// src/utils/password.ts
var SALT_LENGTH = 16;
var ITERATIONS = 1e5;
var KEY_LENGTH = 32;
var ALGORITHM = "PBKDF2";
function generateSalt() {
  return crypto.getRandomValues(new Uint8Array(SALT_LENGTH));
}
__name(generateSalt, "generateSalt");
async function deriveKey(password, salt) {
  const encoder2 = new TextEncoder();
  const passwordKey = await crypto.subtle.importKey(
    "raw",
    encoder2.encode(password),
    ALGORITHM,
    false,
    ["deriveBits"]
  );
  return crypto.subtle.deriveBits(
    {
      name: ALGORITHM,
      salt,
      iterations: ITERATIONS,
      hash: "SHA-256"
    },
    passwordKey,
    KEY_LENGTH * 8
  );
}
__name(deriveKey, "deriveKey");
function bufferToHex(buffer) {
  const bytes = buffer instanceof Uint8Array ? buffer : new Uint8Array(buffer);
  return Array.from(bytes).map((b) => b.toString(16).padStart(2, "0")).join("");
}
__name(bufferToHex, "bufferToHex");
function hexToBuffer(hex) {
  const bytes = new Uint8Array(hex.length / 2);
  for (let i = 0; i < bytes.length; i++) {
    bytes[i] = parseInt(hex.substr(i * 2, 2), 16);
  }
  return bytes;
}
__name(hexToBuffer, "hexToBuffer");
async function hashPassword(password) {
  const salt = generateSalt();
  const hash = await deriveKey(password, salt);
  return `${bufferToHex(salt)}:${bufferToHex(hash)}`;
}
__name(hashPassword, "hashPassword");
async function verifyPassword(password, storedHash) {
  try {
    const [saltHex, hashHex] = storedHash.split(":");
    if (!saltHex || !hashHex) {
      return false;
    }
    const salt = hexToBuffer(saltHex);
    const expectedHash = hexToBuffer(hashHex);
    const derivedHash = await deriveKey(password, salt);
    const derivedArray = new Uint8Array(derivedHash);
    if (derivedArray.length !== expectedHash.length) {
      return false;
    }
    let result = 0;
    for (let i = 0; i < derivedArray.length; i++) {
      result |= derivedArray[i] ^ expectedHash[i];
    }
    return result === 0;
  } catch {
    return false;
  }
}
__name(verifyPassword, "verifyPassword");
function validatePassword(password) {
  if (password.length < 8) {
    return "Password must be at least 8 characters long";
  }
  if (password.length > 128) {
    return "Password must be at most 128 characters long";
  }
  if (!/[a-z]/.test(password)) {
    return "Password must contain at least one lowercase letter";
  }
  if (!/[A-Z]/.test(password)) {
    return "Password must contain at least one uppercase letter";
  }
  if (!/[0-9]/.test(password)) {
    return "Password must contain at least one number";
  }
  return null;
}
__name(validatePassword, "validatePassword");
function validateEmail(email) {
  const emailRegex = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
  return emailRegex.test(email) && email.length <= 255;
}
__name(validateEmail, "validateEmail");

// src/routes/auth.ts
var auth = new Hono2();
auth.post("/register", async (c) => {
  try {
    const body = await c.req.json();
    const { email, password, name } = body;
    if (!email || !password) {
      return c.json({ error: "Email and password are required" }, 400);
    }
    if (!validateEmail(email)) {
      return c.json({ error: "Invalid email format" }, 400);
    }
    const passwordError = validatePassword(password);
    if (passwordError) {
      return c.json({ error: passwordError }, 400);
    }
    const db = c.env.DB;
    const existingUser = await db.prepare("SELECT id FROM users WHERE email = ?").bind(email.toLowerCase()).first();
    if (existingUser) {
      return c.json({ error: "Email already registered" }, 409);
    }
    const userId = generateId3();
    const tenantId = generateId3();
    const sessionId = generateId3();
    const now = Date.now();
    const passwordHash = await hashPassword(password);
    await db.prepare(`
        INSERT INTO users (id, email, password_hash, name, is_active, created_at, last_login)
        VALUES (?, ?, ?, ?, 1, ?, ?)
      `).bind(userId, email.toLowerCase(), passwordHash, name || null, now, now).run();
    const tenantName = name ? `${name}'s Workspace` : "My Workspace";
    await db.prepare(`
        INSERT INTO tenants (id, name, plan, max_memories, max_namespaces, created_at, updated_at)
        VALUES (?, ?, 'free', 1000, 1, ?, ?)
      `).bind(tenantId, tenantName, now, now).run();
    await db.prepare(`
        INSERT INTO user_tenants (user_id, tenant_id, role, created_at)
        VALUES (?, ?, 'owner', ?)
      `).bind(userId, tenantId, now).run();
    await db.prepare(`
        INSERT INTO namespaces (name, tenant_id, dimensions, created_at, updated_at)
        VALUES (?, ?, 1536, ?, ?)
      `).bind(`${tenantId}-default`, tenantId, now, now).run();
    const tenantInfo = [{
      id: tenantId,
      name: tenantName,
      role: "owner"
    }];
    const accessToken = await createAccessToken(
      { sub: userId, email: email.toLowerCase(), name: name || "", tenants: tenantInfo },
      c.env.JWT_SECRET
    );
    const refreshToken = await createRefreshToken(userId, sessionId, c.env.JWT_REFRESH_SECRET);
    const refreshTokenHash = await hashRefreshToken(refreshToken);
    await db.prepare(`
        INSERT INTO sessions (id, user_id, refresh_token_hash, expires_at, created_at)
        VALUES (?, ?, ?, ?, ?)
      `).bind(sessionId, userId, refreshTokenHash, getRefreshTokenExpiry(), now).run();
    return c.json({
      success: true,
      accessToken,
      refreshToken,
      user: {
        id: userId,
        email: email.toLowerCase(),
        name: name || null
      },
      tenants: tenantInfo
    }, 201);
  } catch (error) {
    console.error("Registration error:", error);
    const message2 = error instanceof Error ? error.message : "Unknown error";
    return c.json({ error: "Registration failed", details: message2 }, 500);
  }
});
auth.post("/login", async (c) => {
  try {
    const body = await c.req.json();
    const { email, password } = body;
    if (!email || !password) {
      return c.json({ error: "Email and password are required" }, 400);
    }
    const db = c.env.DB;
    const user = await db.prepare("SELECT id, email, password_hash, name, is_active FROM users WHERE email = ?").bind(email.toLowerCase()).first();
    if (!user) {
      return c.json({ error: "Invalid email or password" }, 401);
    }
    if (!user.is_active) {
      return c.json({ error: "Account is disabled" }, 403);
    }
    const isValid = await verifyPassword(password, user.password_hash);
    if (!isValid) {
      return c.json({ error: "Invalid email or password" }, 401);
    }
    const tenantsResult = await db.prepare(`
        SELECT t.id, t.name, ut.role
        FROM tenants t
        JOIN user_tenants ut ON t.id = ut.tenant_id
        WHERE ut.user_id = ?
      `).bind(user.id).all();
    const tenants2 = tenantsResult.results.map((t) => ({
      id: t.id,
      name: t.name,
      role: t.role
    }));
    const sessionId = generateId3();
    const now = Date.now();
    const accessToken = await createAccessToken(
      { sub: user.id, email: user.email, name: user.name || "", tenants: tenants2 },
      c.env.JWT_SECRET
    );
    const refreshToken = await createRefreshToken(user.id, sessionId, c.env.JWT_REFRESH_SECRET);
    const refreshTokenHash = await hashRefreshToken(refreshToken);
    await db.prepare(`
        INSERT INTO sessions (id, user_id, refresh_token_hash, expires_at, created_at)
        VALUES (?, ?, ?, ?, ?)
      `).bind(sessionId, user.id, refreshTokenHash, getRefreshTokenExpiry(), now).run();
    await db.prepare("UPDATE users SET last_login = ? WHERE id = ?").bind(now, user.id).run();
    return c.json({
      success: true,
      accessToken,
      refreshToken,
      user: {
        id: user.id,
        email: user.email,
        name: user.name
      },
      tenants: tenants2
    });
  } catch (error) {
    console.error("Login error:", error);
    return c.json({ error: "Login failed" }, 500);
  }
});
auth.post("/refresh", async (c) => {
  try {
    const body = await c.req.json();
    const { refreshToken } = body;
    if (!refreshToken) {
      return c.json({ error: "Refresh token is required" }, 400);
    }
    const payload = await verifyRefreshToken(refreshToken, c.env.JWT_REFRESH_SECRET);
    if (!payload) {
      return c.json({ error: "Invalid or expired refresh token" }, 401);
    }
    const db = c.env.DB;
    const tokenHash = await hashRefreshToken(refreshToken);
    const session = await db.prepare(`
        SELECT s.id, s.user_id, s.expires_at, u.email, u.name, u.is_active
        FROM sessions s
        JOIN users u ON s.user_id = u.id
        WHERE s.refresh_token_hash = ? AND s.user_id = ?
      `).bind(tokenHash, payload.sub).first();
    if (!session) {
      return c.json({ error: "Session not found" }, 401);
    }
    if (session.expires_at < Date.now()) {
      await db.prepare("DELETE FROM sessions WHERE id = ?").bind(session.id).run();
      return c.json({ error: "Session expired" }, 401);
    }
    if (!session.is_active) {
      return c.json({ error: "Account is disabled" }, 403);
    }
    const tenantsResult = await db.prepare(`
        SELECT t.id, t.name, ut.role
        FROM tenants t
        JOIN user_tenants ut ON t.id = ut.tenant_id
        WHERE ut.user_id = ?
      `).bind(session.user_id).all();
    const tenants2 = tenantsResult.results.map((t) => ({
      id: t.id,
      name: t.name,
      role: t.role
    }));
    const newSessionId = generateId3();
    const now = Date.now();
    const newAccessToken = await createAccessToken(
      { sub: session.user_id, email: session.email, name: session.name || "", tenants: tenants2 },
      c.env.JWT_SECRET
    );
    const newRefreshToken = await createRefreshToken(session.user_id, newSessionId, c.env.JWT_REFRESH_SECRET);
    const newRefreshTokenHash = await hashRefreshToken(newRefreshToken);
    await db.prepare("DELETE FROM sessions WHERE id = ?").bind(session.id).run();
    await db.prepare(`
        INSERT INTO sessions (id, user_id, refresh_token_hash, expires_at, created_at)
        VALUES (?, ?, ?, ?, ?)
      `).bind(newSessionId, session.user_id, newRefreshTokenHash, getRefreshTokenExpiry(), now).run();
    return c.json({
      success: true,
      accessToken: newAccessToken,
      refreshToken: newRefreshToken
    });
  } catch (error) {
    console.error("Refresh error:", error);
    return c.json({ error: "Token refresh failed" }, 500);
  }
});
auth.post("/logout", async (c) => {
  try {
    const body = await c.req.json();
    const { refreshToken } = body;
    if (!refreshToken) {
      return c.json({ error: "Refresh token is required" }, 400);
    }
    const db = c.env.DB;
    const tokenHash = await hashRefreshToken(refreshToken);
    await db.prepare("DELETE FROM sessions WHERE refresh_token_hash = ?").bind(tokenHash).run();
    return c.json({ success: true, message: "Logged out successfully" });
  } catch (error) {
    console.error("Logout error:", error);
    return c.json({ error: "Logout failed" }, 500);
  }
});
auth.get("/me", requireJwtAuth, async (c) => {
  try {
    const userId = c.get("userId");
    const db = c.env.DB;
    const user = await db.prepare("SELECT id, email, name, created_at, last_login FROM users WHERE id = ?").bind(userId).first();
    if (!user) {
      return c.json({ error: "User not found" }, 404);
    }
    const tenantsResult = await db.prepare(`
        SELECT
          t.id,
          t.name,
          t.plan,
          t.max_memories,
          t.max_namespaces,
          ut.role,
          t.created_at
        FROM tenants t
        JOIN user_tenants ut ON t.id = ut.tenant_id
        WHERE ut.user_id = ?
      `).bind(userId).all();
    return c.json({
      user: {
        id: user.id,
        email: user.email,
        name: user.name,
        createdAt: user.created_at,
        lastLogin: user.last_login
      },
      tenants: tenantsResult.results.map((t) => ({
        id: t.id,
        name: t.name,
        plan: t.plan,
        maxMemories: t.max_memories,
        maxNamespaces: t.max_namespaces,
        role: t.role,
        createdAt: t.created_at
      }))
    });
  } catch (error) {
    console.error("Get profile error:", error);
    return c.json({ error: "Failed to get profile" }, 500);
  }
});
auth.put("/password", requireJwtAuth, async (c) => {
  try {
    const userId = c.get("userId");
    const body = await c.req.json();
    const { currentPassword, newPassword } = body;
    if (!currentPassword || !newPassword) {
      return c.json({ error: "Current password and new password are required" }, 400);
    }
    const passwordError = validatePassword(newPassword);
    if (passwordError) {
      return c.json({ error: passwordError }, 400);
    }
    const db = c.env.DB;
    const user = await db.prepare("SELECT password_hash FROM users WHERE id = ?").bind(userId).first();
    if (!user) {
      return c.json({ error: "User not found" }, 404);
    }
    const isValid = await verifyPassword(currentPassword, user.password_hash);
    if (!isValid) {
      return c.json({ error: "Current password is incorrect" }, 401);
    }
    const newPasswordHash = await hashPassword(newPassword);
    await db.prepare("UPDATE users SET password_hash = ? WHERE id = ?").bind(newPasswordHash, userId).run();
    await db.prepare("DELETE FROM sessions WHERE user_id = ?").bind(userId).run();
    return c.json({ success: true, message: "Password changed successfully" });
  } catch (error) {
    console.error("Change password error:", error);
    return c.json({ error: "Failed to change password" }, 500);
  }
});
auth.post("/validate-agent", async (c) => {
  try {
    const body = await c.req.json();
    const { apiKey, agentToken } = body;
    if (!apiKey || typeof apiKey !== "string") {
      return c.json({ valid: false, error: "apiKey is required" }, 400);
    }
    if (!agentToken || typeof agentToken !== "string") {
      return c.json({ valid: false, error: "agentToken is required" }, 400);
    }
    const db = c.env.DB;
    const service = new AgentTokenService(db);
    const result = await service.validate(apiKey, agentToken);
    if (!result.valid) {
      return c.json({ valid: false, error: result.error }, 401);
    }
    if (result.agentTokenId) {
      await service.recordUsage(result.agentTokenId);
    }
    return c.json({
      valid: true,
      userId: result.userId,
      tenantId: result.tenantId,
      agentTokenId: result.agentTokenId,
      agentName: result.agentName,
      allowedMemories: result.allowedMemories,
      permissions: result.permissions,
      expiresAt: result.expiresAt
    });
  } catch (error) {
    console.error("Validate agent error:", error);
    return c.json({ valid: false, error: "Validation failed" }, 500);
  }
});
var auth_default = auth;

// src/routes/tenants.ts
var tenants = new Hono2();
tenants.use("/*", requireJwtAuth);
tenants.get("/", async (c) => {
  try {
    const userId = c.get("userId");
    const db = c.env.DB;
    const tenantsResult = await db.prepare(`
        SELECT
          t.id,
          t.name,
          t.plan,
          t.max_memories,
          t.max_namespaces,
          t.created_at,
          t.updated_at,
          ut.role,
          (SELECT COUNT(*) FROM namespaces WHERE tenant_id = t.id) as namespace_count,
          (SELECT COUNT(*) FROM user_tenants WHERE tenant_id = t.id) as member_count
        FROM tenants t
        JOIN user_tenants ut ON t.id = ut.tenant_id
        WHERE ut.user_id = ?
        ORDER BY t.created_at DESC
      `).bind(userId).all();
    return c.json({
      tenants: tenantsResult.results.map((t) => ({
        id: t.id,
        name: t.name,
        plan: t.plan,
        maxMemories: t.max_memories,
        maxNamespaces: t.max_namespaces,
        role: t.role,
        namespaceCount: t.namespace_count,
        memberCount: t.member_count,
        createdAt: t.created_at,
        updatedAt: t.updated_at
      }))
    });
  } catch (error) {
    console.error("List tenants error:", error);
    return c.json({ error: "Failed to list tenants" }, 500);
  }
});
tenants.post("/", async (c) => {
  try {
    const userId = c.get("userId");
    const body = await c.req.json();
    const { name } = body;
    if (!name || name.trim().length === 0) {
      return c.json({ error: "Tenant name is required" }, 400);
    }
    if (name.length > 100) {
      return c.json({ error: "Tenant name must be 100 characters or less" }, 400);
    }
    const db = c.env.DB;
    const tenantId = generateId3();
    const now = Date.now();
    await db.prepare(`
        INSERT INTO tenants (id, name, plan, max_memories, max_namespaces, created_at, updated_at)
        VALUES (?, ?, 'free', 1000, 1, ?, ?)
      `).bind(tenantId, name.trim(), now, now).run();
    await db.prepare(`
        INSERT INTO user_tenants (user_id, tenant_id, role, created_at)
        VALUES (?, ?, 'owner', ?)
      `).bind(userId, tenantId, now).run();
    await db.prepare(`
        INSERT INTO namespaces (name, tenant_id, dimensions, created_at, updated_at)
        VALUES (?, ?, 1536, ?, ?)
      `).bind(`${tenantId}-default`, tenantId, now, now).run();
    return c.json({
      success: true,
      tenant: {
        id: tenantId,
        name: name.trim(),
        plan: "free",
        maxMemories: 1e3,
        maxNamespaces: 1,
        role: "owner",
        createdAt: now,
        updatedAt: now
      }
    }, 201);
  } catch (error) {
    console.error("Create tenant error:", error);
    return c.json({ error: "Failed to create tenant" }, 500);
  }
});
tenants.get("/:id", async (c) => {
  try {
    const userId = c.get("userId");
    const tenantId = c.req.param("id");
    const db = c.env.DB;
    const membership = await db.prepare("SELECT role FROM user_tenants WHERE user_id = ? AND tenant_id = ?").bind(userId, tenantId).first();
    if (!membership) {
      return c.json({ error: "Tenant not found or access denied" }, 404);
    }
    const tenant = await db.prepare(`
        SELECT
          t.*,
          (SELECT COUNT(*) FROM namespaces WHERE tenant_id = t.id) as namespace_count,
          (SELECT COUNT(*) FROM user_tenants WHERE tenant_id = t.id) as member_count,
          (SELECT COUNT(*) FROM memories m JOIN namespaces n ON m.namespace = n.name WHERE n.tenant_id = t.id) as memory_count
        FROM tenants t
        WHERE t.id = ?
      `).bind(tenantId).first();
    if (!tenant) {
      return c.json({ error: "Tenant not found" }, 404);
    }
    const namespacesResult = await db.prepare(`
        SELECT
          n.name,
          n.dimensions,
          n.created_at,
          (SELECT COUNT(*) FROM memories WHERE namespace = n.name) as memory_count
        FROM namespaces n
        WHERE n.tenant_id = ?
        ORDER BY n.created_at DESC
      `).bind(tenantId).all();
    return c.json({
      tenant: {
        id: tenant.id,
        name: tenant.name,
        plan: tenant.plan,
        maxMemories: tenant.max_memories,
        maxNamespaces: tenant.max_namespaces,
        role: membership.role,
        stats: {
          namespaces: tenant.namespace_count,
          members: tenant.member_count,
          memories: tenant.memory_count
        },
        createdAt: tenant.created_at,
        updatedAt: tenant.updated_at
      },
      namespaces: namespacesResult.results.map((n) => ({
        name: n.name,
        dimensions: n.dimensions,
        memoryCount: n.memory_count,
        createdAt: n.created_at
      }))
    });
  } catch (error) {
    console.error("Get tenant error:", error);
    return c.json({ error: "Failed to get tenant" }, 500);
  }
});
tenants.put("/:id", async (c) => {
  try {
    const userId = c.get("userId");
    const tenantId = c.req.param("id");
    const body = await c.req.json();
    const { name } = body;
    const db = c.env.DB;
    const membership = await db.prepare("SELECT role FROM user_tenants WHERE user_id = ? AND tenant_id = ?").bind(userId, tenantId).first();
    if (!membership || !["owner", "admin"].includes(membership.role)) {
      return c.json({ error: "Access denied. Owner or admin role required." }, 403);
    }
    if (!name || name.trim().length === 0) {
      return c.json({ error: "Tenant name is required" }, 400);
    }
    if (name.length > 100) {
      return c.json({ error: "Tenant name must be 100 characters or less" }, 400);
    }
    const now = Date.now();
    await db.prepare("UPDATE tenants SET name = ?, updated_at = ? WHERE id = ?").bind(name.trim(), now, tenantId).run();
    return c.json({
      success: true,
      tenant: {
        id: tenantId,
        name: name.trim(),
        updatedAt: now
      }
    });
  } catch (error) {
    console.error("Update tenant error:", error);
    return c.json({ error: "Failed to update tenant" }, 500);
  }
});
tenants.delete("/:id", async (c) => {
  try {
    const userId = c.get("userId");
    const tenantId = c.req.param("id");
    const db = c.env.DB;
    const membership = await db.prepare("SELECT role FROM user_tenants WHERE user_id = ? AND tenant_id = ?").bind(userId, tenantId).first();
    if (!membership || membership.role !== "owner") {
      return c.json({ error: "Access denied. Owner role required." }, 403);
    }
    const tenantCount = await db.prepare("SELECT COUNT(*) as count FROM user_tenants WHERE user_id = ?").bind(userId).first();
    if (tenantCount && tenantCount.count <= 1) {
      return c.json({ error: "Cannot delete your last tenant" }, 400);
    }
    await db.prepare("DELETE FROM tenants WHERE id = ?").bind(tenantId).run();
    return c.json({ success: true, message: "Tenant deleted successfully" });
  } catch (error) {
    console.error("Delete tenant error:", error);
    return c.json({ error: "Failed to delete tenant" }, 500);
  }
});
tenants.get("/:id/members", async (c) => {
  try {
    const userId = c.get("userId");
    const tenantId = c.req.param("id");
    const db = c.env.DB;
    const membership = await db.prepare("SELECT role FROM user_tenants WHERE user_id = ? AND tenant_id = ?").bind(userId, tenantId).first();
    if (!membership) {
      return c.json({ error: "Tenant not found or access denied" }, 404);
    }
    const membersResult = await db.prepare(`
        SELECT u.id, u.email, u.name, ut.role, ut.created_at
        FROM users u
        JOIN user_tenants ut ON u.id = ut.user_id
        WHERE ut.tenant_id = ?
        ORDER BY ut.created_at ASC
      `).bind(tenantId).all();
    return c.json({
      members: membersResult.results.map((m) => ({
        id: m.id,
        email: m.email,
        name: m.name,
        role: m.role,
        joinedAt: m.created_at
      }))
    });
  } catch (error) {
    console.error("List members error:", error);
    return c.json({ error: "Failed to list members" }, 500);
  }
});
tenants.post("/:id/members", async (c) => {
  try {
    const userId = c.get("userId");
    const tenantId = c.req.param("id");
    const body = await c.req.json();
    const { email, role = "member" } = body;
    const db = c.env.DB;
    const validRoles = ["admin", "member", "viewer"];
    if (!validRoles.includes(role)) {
      return c.json({ error: "Invalid role. Must be admin, member, or viewer." }, 400);
    }
    const membership = await db.prepare("SELECT role FROM user_tenants WHERE user_id = ? AND tenant_id = ?").bind(userId, tenantId).first();
    if (!membership || !["owner", "admin"].includes(membership.role)) {
      return c.json({ error: "Access denied. Owner or admin role required." }, 403);
    }
    const targetUser = await db.prepare("SELECT id, email, name FROM users WHERE email = ?").bind(email.toLowerCase()).first();
    if (!targetUser) {
      return c.json({ error: "User not found. They must register first." }, 404);
    }
    const existingMembership = await db.prepare("SELECT role FROM user_tenants WHERE user_id = ? AND tenant_id = ?").bind(targetUser.id, tenantId).first();
    if (existingMembership) {
      return c.json({ error: "User is already a member of this tenant" }, 409);
    }
    const now = Date.now();
    await db.prepare(`
        INSERT INTO user_tenants (user_id, tenant_id, role, created_at)
        VALUES (?, ?, ?, ?)
      `).bind(targetUser.id, tenantId, role, now).run();
    return c.json({
      success: true,
      member: {
        id: targetUser.id,
        email: targetUser.email,
        name: targetUser.name,
        role,
        joinedAt: now
      }
    }, 201);
  } catch (error) {
    console.error("Invite member error:", error);
    return c.json({ error: "Failed to invite member" }, 500);
  }
});
tenants.put("/:id/members/:memberId", async (c) => {
  try {
    const userId = c.get("userId");
    const tenantId = c.req.param("id");
    const memberId = c.req.param("memberId");
    const body = await c.req.json();
    const { role } = body;
    const db = c.env.DB;
    const validRoles = ["admin", "member", "viewer"];
    if (!validRoles.includes(role)) {
      return c.json({ error: "Invalid role. Must be admin, member, or viewer." }, 400);
    }
    const membership = await db.prepare("SELECT role FROM user_tenants WHERE user_id = ? AND tenant_id = ?").bind(userId, tenantId).first();
    if (!membership || membership.role !== "owner") {
      return c.json({ error: "Access denied. Owner role required." }, 403);
    }
    const targetMembership = await db.prepare("SELECT role FROM user_tenants WHERE user_id = ? AND tenant_id = ?").bind(memberId, tenantId).first();
    if (!targetMembership) {
      return c.json({ error: "Member not found" }, 404);
    }
    if (targetMembership.role === "owner") {
      return c.json({ error: "Cannot change owner role" }, 400);
    }
    await db.prepare("UPDATE user_tenants SET role = ? WHERE user_id = ? AND tenant_id = ?").bind(role, memberId, tenantId).run();
    return c.json({
      success: true,
      message: "Member role updated",
      role
    });
  } catch (error) {
    console.error("Update member error:", error);
    return c.json({ error: "Failed to update member" }, 500);
  }
});
tenants.delete("/:id/members/:memberId", async (c) => {
  try {
    const userId = c.get("userId");
    const tenantId = c.req.param("id");
    const memberId = c.req.param("memberId");
    const db = c.env.DB;
    const membership = await db.prepare("SELECT role FROM user_tenants WHERE user_id = ? AND tenant_id = ?").bind(userId, tenantId).first();
    if (!membership || !["owner", "admin"].includes(membership.role)) {
      return c.json({ error: "Access denied. Owner or admin role required." }, 403);
    }
    const targetMembership = await db.prepare("SELECT role FROM user_tenants WHERE user_id = ? AND tenant_id = ?").bind(memberId, tenantId).first();
    if (!targetMembership) {
      return c.json({ error: "Member not found" }, 404);
    }
    if (targetMembership.role === "owner") {
      return c.json({ error: "Cannot remove tenant owner" }, 400);
    }
    if (membership.role === "admin" && targetMembership.role === "admin") {
      return c.json({ error: "Admins cannot remove other admins" }, 403);
    }
    await db.prepare("DELETE FROM user_tenants WHERE user_id = ? AND tenant_id = ?").bind(memberId, tenantId).run();
    return c.json({ success: true, message: "Member removed" });
  } catch (error) {
    console.error("Remove member error:", error);
    return c.json({ error: "Failed to remove member" }, 500);
  }
});
var tenants_default = tenants;

// src/routes/audit.ts
var auditRoutes = new Hono2();
function getAuditService(c) {
  if (!c.env?.DB)
    return null;
  return new AuditService(c.env.DB);
}
__name(getAuditService, "getAuditService");
auditRoutes.get("/", async (c) => {
  const auditService = getAuditService(c);
  if (!auditService) {
    return c.json({ error: "Audit logging requires D1 database" }, 503);
  }
  try {
    const query = c.req.query();
    const options = {
      action: query.action,
      resourceType: query.resourceType,
      resourceId: query.resourceId,
      userId: query.userId,
      tenantId: query.tenantId || c.req.header("X-Tenant-Id"),
      namespace: query.namespace || c.req.header("X-Namespace"),
      startTime: query.startTime ? parseInt(query.startTime) : void 0,
      endTime: query.endTime ? parseInt(query.endTime) : void 0,
      success: query.success === "true" ? true : query.success === "false" ? false : void 0,
      requestId: query.requestId,
      limit: query.limit ? parseInt(query.limit) : 100,
      offset: query.offset ? parseInt(query.offset) : 0
    };
    const result = await auditService.query(options);
    return c.json({
      success: true,
      ...result
    });
  } catch (error) {
    return c.json({
      error: error instanceof Error ? error.message : "Unknown error"
    }, 500);
  }
});
auditRoutes.get("/:id", async (c) => {
  const auditService = getAuditService(c);
  if (!auditService) {
    return c.json({ error: "Audit logging requires D1 database" }, 503);
  }
  try {
    const id = c.req.param("id");
    const entry = await auditService.getById(id);
    if (!entry) {
      return c.json({ error: "Audit entry not found" }, 404);
    }
    return c.json({
      success: true,
      entry
    });
  } catch (error) {
    return c.json({
      error: error instanceof Error ? error.message : "Unknown error"
    }, 500);
  }
});
auditRoutes.get("/resource/:type/:id", async (c) => {
  const auditService = getAuditService(c);
  if (!auditService) {
    return c.json({ error: "Audit logging requires D1 database" }, 503);
  }
  try {
    const resourceType = c.req.param("type");
    const resourceId = c.req.param("id");
    const limit = parseInt(c.req.query("limit") || "50");
    const entries = await auditService.getResourceHistory(resourceType, resourceId, limit);
    return c.json({
      success: true,
      resourceType,
      resourceId,
      entries,
      count: entries.length
    });
  } catch (error) {
    return c.json({
      error: error instanceof Error ? error.message : "Unknown error"
    }, 500);
  }
});
auditRoutes.get("/user/:id", async (c) => {
  const auditService = getAuditService(c);
  if (!auditService) {
    return c.json({ error: "Audit logging requires D1 database" }, 503);
  }
  try {
    const userId = c.req.param("id");
    const query = c.req.query();
    const entries = await auditService.getUserActivity(userId, {
      startTime: query.startTime ? parseInt(query.startTime) : void 0,
      endTime: query.endTime ? parseInt(query.endTime) : void 0,
      limit: query.limit ? parseInt(query.limit) : 100
    });
    return c.json({
      success: true,
      userId,
      entries,
      count: entries.length
    });
  } catch (error) {
    return c.json({
      error: error instanceof Error ? error.message : "Unknown error"
    }, 500);
  }
});
auditRoutes.get("/failures", async (c) => {
  const auditService = getAuditService(c);
  if (!auditService) {
    return c.json({ error: "Audit logging requires D1 database" }, 503);
  }
  try {
    const query = c.req.query();
    const entries = await auditService.getFailures({
      tenantId: query.tenantId || c.req.header("X-Tenant-Id"),
      namespace: query.namespace || c.req.header("X-Namespace"),
      limit: query.limit ? parseInt(query.limit) : 50
    });
    return c.json({
      success: true,
      entries,
      count: entries.length
    });
  } catch (error) {
    return c.json({
      error: error instanceof Error ? error.message : "Unknown error"
    }, 500);
  }
});
auditRoutes.get("/stats", async (c) => {
  const auditService = getAuditService(c);
  if (!auditService) {
    return c.json({ error: "Audit logging requires D1 database" }, 503);
  }
  try {
    const query = c.req.query();
    const tenantId = query.tenantId || c.req.header("X-Tenant-Id");
    const stats = await auditService.getStats(tenantId, {
      startTime: query.startTime ? parseInt(query.startTime) : void 0,
      endTime: query.endTime ? parseInt(query.endTime) : void 0
    });
    return c.json({
      success: true,
      tenantId,
      stats
    });
  } catch (error) {
    return c.json({
      error: error instanceof Error ? error.message : "Unknown error"
    }, 500);
  }
});
auditRoutes.post("/cleanup", async (c) => {
  const auditService = getAuditService(c);
  if (!auditService) {
    return c.json({ error: "Audit logging requires D1 database" }, 503);
  }
  try {
    const body = await c.req.json().catch(() => ({}));
    const retentionDays = body.retentionDays || 90;
    const deletedCount = await auditService.cleanup(retentionDays);
    return c.json({
      success: true,
      deletedCount,
      message: `Deleted ${deletedCount} audit entries older than ${retentionDays} days`
    });
  } catch (error) {
    return c.json({
      error: error instanceof Error ? error.message : "Unknown error"
    }, 500);
  }
});
var audit_default = auditRoutes;

// src/routes/knowledge.ts
var knowledgeRoutes = new Hono2();
function getNamespace(c) {
  return c.req.header("X-Namespace") || "default";
}
__name(getNamespace, "getNamespace");
function getAuditContext(c) {
  return {
    namespace: getNamespace(c),
    userId: c.req.header("X-User-Id"),
    tenantId: c.req.header("X-Tenant-Id"),
    apiKey: c.req.header("X-API-Key"),
    ipAddress: c.req.header("CF-Connecting-IP") || c.req.header("X-Forwarded-For"),
    userAgent: c.req.header("User-Agent"),
    requestId: c.req.header("X-Request-Id")
  };
}
__name(getAuditContext, "getAuditContext");
knowledgeRoutes.post("/ingest", async (c) => {
  const startTime = Date.now();
  const namespace = getNamespace(c);
  try {
    const body = await c.req.json();
    const {
      content,
      name,
      type = "document",
      url,
      mimeType,
      metadata = {},
      chunking = {},
      generateEmbeddings = true
    } = body;
    if (!content || typeof content !== "string") {
      return c.json({ error: "content is required and must be a string" }, 400);
    }
    if (!name || typeof name !== "string") {
      return c.json({ error: "name is required and must be a string" }, 400);
    }
    const knowledgeService = new KnowledgeService(c.env?.DB || null);
    if (!knowledgeService.isAvailable()) {
      return c.json({ error: "Knowledge service requires D1 database" }, 503);
    }
    const chunks = knowledgeService.chunkText(content, chunking);
    if (chunks.length === 0) {
      return c.json({ error: "No valid chunks could be created from content" }, 400);
    }
    const source = await knowledgeService.createSource(namespace, {
      name,
      type,
      url,
      mimeType,
      size: content.length,
      chunkCount: chunks.length,
      namespace,
      metadata
    });
    let embeddingsGenerated = false;
    let chunkEmbeddings = [];
    if (generateEmbeddings && c.env?.AI) {
      const embeddingService = new EmbeddingService(c.env.AI);
      const texts = chunks.map((chunk) => chunk.text);
      try {
        const result2 = await embeddingService.embedBatch(texts, { dimensions: 768 });
        chunkEmbeddings = result2.embeddings;
        embeddingsGenerated = true;
      } catch (error) {
        console.error("Failed to generate embeddings:", error);
      }
    }
    const db = c.env?.DB;
    if (db) {
      for (let i = 0; i < chunks.length; i++) {
        const chunk = chunks[i];
        const embedding = chunkEmbeddings[i] || [];
        const memoryId = `mem_${source.id}_${i}`;
        await db.prepare(`
						INSERT INTO memories
						(id, namespace, type, content, embedding, importance, metadata, created_at, updated_at, access_count)
						VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
					`).bind(
          memoryId,
          namespace,
          "knowledge",
          chunk.text,
          JSON.stringify(embedding),
          0.5,
          // Default importance for knowledge
          JSON.stringify({
            sourceId: source.id,
            sourceName: name,
            sourceType: type,
            sourceUrl: url,
            chunkIndex: i,
            totalChunks: chunks.length,
            startOffset: chunk.startOffset,
            endOffset: chunk.endOffset,
            ...metadata
          }),
          Date.now(),
          Date.now(),
          0
        ).run();
      }
    }
    if (c.env?.DB) {
      const logger2 = createAuditLogger(c.env.DB, getAuditContext(c));
      await logger2.logMemory("create", source.id, {
        action: "ingest",
        sourceName: name,
        sourceType: type,
        chunksCreated: chunks.length,
        embeddingsGenerated,
        contentLength: content.length
      });
      const tenantId = c.req.header("X-Tenant-Id");
      const webhookTrigger = createWebhookTrigger(c.env.DB, c.executionCtx);
      webhookTrigger(namespace, "knowledge.ingested", {
        sourceId: source.id,
        sourceName: name,
        sourceType: type,
        chunksCreated: chunks.length,
        totalCharacters: content.length
      }, tenantId);
    }
    const result = {
      sourceId: source.id,
      sourceName: name,
      chunksCreated: chunks.length,
      embeddingsGenerated,
      totalCharacters: content.length,
      averageChunkSize: Math.round(content.length / chunks.length)
    };
    return c.json({
      success: true,
      ...result,
      durationMs: Date.now() - startTime
    });
  } catch (error) {
    return c.json({
      error: error instanceof Error ? error.message : "Unknown error"
    }, 500);
  }
});
knowledgeRoutes.get("/sources", async (c) => {
  const namespace = getNamespace(c);
  const type = c.req.query("type");
  const limit = parseInt(c.req.query("limit") || "100");
  const offset = parseInt(c.req.query("offset") || "0");
  const knowledgeService = new KnowledgeService(c.env?.DB || null);
  if (!knowledgeService.isAvailable()) {
    return c.json({ error: "Knowledge service requires D1 database" }, 503);
  }
  const { sources, total } = await knowledgeService.listSources(namespace, {
    type,
    limit,
    offset
  });
  return c.json({
    success: true,
    sources,
    total,
    hasMore: offset + sources.length < total
  });
});
knowledgeRoutes.get("/sources/:id", async (c) => {
  const id = c.req.param("id");
  const knowledgeService = new KnowledgeService(c.env?.DB || null);
  if (!knowledgeService.isAvailable()) {
    return c.json({ error: "Knowledge service requires D1 database" }, 503);
  }
  const source = await knowledgeService.getSource(id);
  if (!source) {
    return c.json({ error: "Source not found" }, 404);
  }
  return c.json({
    success: true,
    source
  });
});
knowledgeRoutes.delete("/sources/:id", async (c) => {
  const id = c.req.param("id");
  const knowledgeService = new KnowledgeService(c.env?.DB || null);
  if (!knowledgeService.isAvailable()) {
    return c.json({ error: "Knowledge service requires D1 database" }, 503);
  }
  const source = await knowledgeService.getSource(id);
  if (!source) {
    return c.json({ error: "Source not found" }, 404);
  }
  const deleted = await knowledgeService.deleteSource(id);
  if (c.env?.DB) {
    const logger2 = createAuditLogger(c.env.DB, getAuditContext(c));
    await logger2.logMemory("delete", id, {
      action: "deleteSource",
      sourceName: source.name,
      chunksDeleted: source.chunkCount
    });
    if (deleted) {
      const namespace = getNamespace(c);
      const tenantId = c.req.header("X-Tenant-Id");
      const webhookTrigger = createWebhookTrigger(c.env.DB, c.executionCtx);
      webhookTrigger(namespace, "knowledge.deleted", {
        sourceId: id,
        sourceName: source.name,
        chunksDeleted: source.chunkCount
      }, tenantId);
    }
  }
  return c.json({
    success: deleted,
    message: deleted ? `Source "${source.name}" and ${source.chunkCount} chunks deleted` : "Delete failed"
  });
});
knowledgeRoutes.get("/stats", async (c) => {
  const namespace = getNamespace(c);
  const knowledgeService = new KnowledgeService(c.env?.DB || null);
  if (!knowledgeService.isAvailable()) {
    return c.json({ error: "Knowledge service requires D1 database" }, 503);
  }
  const stats = await knowledgeService.getStats(namespace);
  return c.json({
    success: true,
    namespace,
    stats
  });
});
knowledgeRoutes.post("/chunk-preview", async (c) => {
  try {
    const body = await c.req.json();
    const { content, chunking = {} } = body;
    if (!content || typeof content !== "string") {
      return c.json({ error: "content is required and must be a string" }, 400);
    }
    const knowledgeService = new KnowledgeService(null);
    const chunks = knowledgeService.chunkText(content, chunking);
    return c.json({
      success: true,
      totalChunks: chunks.length,
      totalCharacters: content.length,
      averageChunkSize: chunks.length > 0 ? Math.round(content.length / chunks.length) : 0,
      chunks: chunks.map((chunk, i) => ({
        index: i,
        length: chunk.text.length,
        preview: chunk.text.substring(0, 100) + (chunk.text.length > 100 ? "..." : ""),
        startOffset: chunk.startOffset,
        endOffset: chunk.endOffset
      }))
    });
  } catch (error) {
    return c.json({
      error: error instanceof Error ? error.message : "Unknown error"
    }, 500);
  }
});
knowledgeRoutes.get("/sources/:id/chunks", async (c) => {
  const id = c.req.param("id");
  const namespace = getNamespace(c);
  const limit = parseInt(c.req.query("limit") || "100");
  const offset = parseInt(c.req.query("offset") || "0");
  if (!c.env?.DB) {
    return c.json({ error: "D1 database required" }, 503);
  }
  const knowledgeService = new KnowledgeService(c.env.DB);
  const source = await knowledgeService.getSource(id);
  if (!source) {
    return c.json({ error: "Source not found" }, 404);
  }
  const results = await c.env.DB.prepare(`
			SELECT id, content, metadata, created_at
			FROM memories
			WHERE namespace = ? AND type = 'knowledge'
			AND metadata LIKE ?
			ORDER BY id
			LIMIT ? OFFSET ?
		`).bind(namespace, `%"sourceId":"${id}"%`, limit, offset).all();
  const chunks = (results.results || []).map((row) => {
    const metadata = JSON.parse(row.metadata || "{}");
    return {
      id: row.id,
      content: row.content,
      chunkIndex: metadata.chunkIndex,
      startOffset: metadata.startOffset,
      endOffset: metadata.endOffset,
      createdAt: row.created_at
    };
  });
  return c.json({
    success: true,
    source: {
      id: source.id,
      name: source.name,
      type: source.type
    },
    chunks,
    total: source.chunkCount,
    hasMore: offset + chunks.length < source.chunkCount
  });
});
var knowledge_default = knowledgeRoutes;

// src/routes/webhooks.ts
var webhookRoutes = new Hono2();
function getNamespace2(c) {
  return c.req.header("X-Namespace") || "default";
}
__name(getNamespace2, "getNamespace");
function getTenantId(c) {
  return c.req.header("X-Tenant-Id");
}
__name(getTenantId, "getTenantId");
function validateEvents(events) {
  if (!Array.isArray(events))
    return null;
  if (events.length === 0)
    return null;
  for (const event of events) {
    if (typeof event !== "string")
      return null;
    if (!WEBHOOK_EVENTS.includes(event))
      return null;
  }
  return events;
}
__name(validateEvents, "validateEvents");
function isValidUrl(url) {
  try {
    const parsed = new URL(url);
    return parsed.protocol === "https:" || parsed.protocol === "http:";
  } catch {
    return false;
  }
}
__name(isValidUrl, "isValidUrl");
webhookRoutes.get("/events", (c) => {
  return c.json({
    events: WEBHOOK_EVENTS.map((event) => ({
      type: event,
      description: getEventDescription(event)
    }))
  });
});
webhookRoutes.get("/", async (c) => {
  const namespace = getNamespace2(c);
  if (!c.env?.DB) {
    return c.json({ error: "Database not available" }, 503);
  }
  const service = new WebhookService(c.env.DB);
  const activeOnly = c.req.query("active") === "true";
  const limit = parseInt(c.req.query("limit") || "100", 10);
  const offset = parseInt(c.req.query("offset") || "0", 10);
  const result = await service.list(namespace, { activeOnly, limit, offset });
  const webhooks = result.webhooks.map((wh) => ({
    ...wh,
    secret: void 0,
    secretPrefix: wh.secret.substring(0, 8) + "..."
  }));
  return c.json({
    webhooks,
    total: result.total,
    hasMore: result.hasMore
  });
});
webhookRoutes.post("/", async (c) => {
  const namespace = getNamespace2(c);
  const tenantId = getTenantId(c);
  if (!c.env?.DB) {
    return c.json({ error: "Database not available" }, 503);
  }
  try {
    const body = await c.req.json();
    const {
      url,
      events,
      description,
      maxRetries = 3,
      retryBackoffMs = 1e3
    } = body;
    if (!url || typeof url !== "string") {
      return c.json({ error: "url is required" }, 400);
    }
    if (!isValidUrl(url)) {
      return c.json({ error: "url must be a valid HTTP/HTTPS URL" }, 400);
    }
    const validatedEvents = validateEvents(events);
    if (!validatedEvents) {
      return c.json({
        error: "events must be a non-empty array of valid event types",
        validEvents: WEBHOOK_EVENTS
      }, 400);
    }
    if (maxRetries < 0 || maxRetries > 10) {
      return c.json({ error: "maxRetries must be between 0 and 10" }, 400);
    }
    if (retryBackoffMs < 100 || retryBackoffMs > 6e4) {
      return c.json({ error: "retryBackoffMs must be between 100 and 60000" }, 400);
    }
    const service = new WebhookService(c.env.DB);
    const options = {
      namespace,
      tenantId,
      url,
      events: validatedEvents,
      description,
      maxRetries,
      retryBackoffMs
    };
    const webhook = await service.create(options);
    return c.json({
      webhook: {
        ...webhook
        // Return full secret only on creation
      },
      message: "Webhook created. Save the secret - it will not be shown again."
    }, 201);
  } catch (error) {
    console.error("Error creating webhook:", error);
    return c.json({ error: "Failed to create webhook" }, 500);
  }
});
webhookRoutes.get("/:id", async (c) => {
  const id = c.req.param("id");
  if (!c.env?.DB) {
    return c.json({ error: "Database not available" }, 503);
  }
  const service = new WebhookService(c.env.DB);
  const webhook = await service.get(id);
  if (!webhook) {
    return c.json({ error: "Webhook not found" }, 404);
  }
  const namespace = getNamespace2(c);
  if (webhook.namespace !== namespace) {
    return c.json({ error: "Webhook not found" }, 404);
  }
  return c.json({
    webhook: {
      ...webhook,
      secret: void 0,
      secretPrefix: webhook.secret.substring(0, 8) + "..."
    }
  });
});
webhookRoutes.put("/:id", async (c) => {
  const id = c.req.param("id");
  const namespace = getNamespace2(c);
  if (!c.env?.DB) {
    return c.json({ error: "Database not available" }, 503);
  }
  const service = new WebhookService(c.env.DB);
  const existing = await service.get(id);
  if (!existing) {
    return c.json({ error: "Webhook not found" }, 404);
  }
  if (existing.namespace !== namespace) {
    return c.json({ error: "Webhook not found" }, 404);
  }
  try {
    const body = await c.req.json();
    const updates = {};
    if (body.url !== void 0) {
      if (typeof body.url !== "string" || !isValidUrl(body.url)) {
        return c.json({ error: "url must be a valid HTTP/HTTPS URL" }, 400);
      }
      updates.url = body.url;
    }
    if (body.events !== void 0) {
      const validatedEvents = validateEvents(body.events);
      if (!validatedEvents) {
        return c.json({
          error: "events must be a non-empty array of valid event types",
          validEvents: WEBHOOK_EVENTS
        }, 400);
      }
      updates.events = validatedEvents;
    }
    if (body.isActive !== void 0) {
      updates.isActive = Boolean(body.isActive);
    }
    if (body.description !== void 0) {
      updates.description = body.description;
    }
    if (body.maxRetries !== void 0) {
      if (body.maxRetries < 0 || body.maxRetries > 10) {
        return c.json({ error: "maxRetries must be between 0 and 10" }, 400);
      }
      updates.maxRetries = body.maxRetries;
    }
    if (body.retryBackoffMs !== void 0) {
      if (body.retryBackoffMs < 100 || body.retryBackoffMs > 6e4) {
        return c.json({ error: "retryBackoffMs must be between 100 and 60000" }, 400);
      }
      updates.retryBackoffMs = body.retryBackoffMs;
    }
    const webhook = await service.update(id, updates);
    return c.json({
      webhook: {
        ...webhook,
        secret: void 0,
        secretPrefix: webhook?.secret.substring(0, 8) + "..."
      }
    });
  } catch (error) {
    console.error("Error updating webhook:", error);
    return c.json({ error: "Failed to update webhook" }, 500);
  }
});
webhookRoutes.delete("/:id", async (c) => {
  const id = c.req.param("id");
  const namespace = getNamespace2(c);
  if (!c.env?.DB) {
    return c.json({ error: "Database not available" }, 503);
  }
  const service = new WebhookService(c.env.DB);
  const existing = await service.get(id);
  if (!existing) {
    return c.json({ error: "Webhook not found" }, 404);
  }
  if (existing.namespace !== namespace) {
    return c.json({ error: "Webhook not found" }, 404);
  }
  const deleted = await service.delete(id);
  if (!deleted) {
    return c.json({ error: "Failed to delete webhook" }, 500);
  }
  return c.json({ success: true });
});
webhookRoutes.post("/:id/test", async (c) => {
  const id = c.req.param("id");
  const namespace = getNamespace2(c);
  if (!c.env?.DB) {
    return c.json({ error: "Database not available" }, 503);
  }
  const service = new WebhookService(c.env.DB);
  const webhook = await service.get(id);
  if (!webhook) {
    return c.json({ error: "Webhook not found" }, 404);
  }
  if (webhook.namespace !== namespace) {
    return c.json({ error: "Webhook not found" }, 404);
  }
  const result = await service.test(id);
  return c.json({
    success: result.success,
    status: result.status,
    error: result.error
  });
});
webhookRoutes.post("/:id/rotate-secret", async (c) => {
  const id = c.req.param("id");
  const namespace = getNamespace2(c);
  if (!c.env?.DB) {
    return c.json({ error: "Database not available" }, 503);
  }
  const service = new WebhookService(c.env.DB);
  const webhook = await service.get(id);
  if (!webhook) {
    return c.json({ error: "Webhook not found" }, 404);
  }
  if (webhook.namespace !== namespace) {
    return c.json({ error: "Webhook not found" }, 404);
  }
  const newSecret = await service.rotateSecret(id);
  if (!newSecret) {
    return c.json({ error: "Failed to rotate secret" }, 500);
  }
  return c.json({
    secret: newSecret,
    message: "Secret rotated. Update your webhook handler with the new secret."
  });
});
webhookRoutes.get("/:id/deliveries", async (c) => {
  const id = c.req.param("id");
  const namespace = getNamespace2(c);
  if (!c.env?.DB) {
    return c.json({ error: "Database not available" }, 503);
  }
  const service = new WebhookService(c.env.DB);
  const webhook = await service.get(id);
  if (!webhook) {
    return c.json({ error: "Webhook not found" }, 404);
  }
  if (webhook.namespace !== namespace) {
    return c.json({ error: "Webhook not found" }, 404);
  }
  const status = c.req.query("status");
  const limit = parseInt(c.req.query("limit") || "100", 10);
  const offset = parseInt(c.req.query("offset") || "0", 10);
  const result = await service.getDeliveries({
    webhookId: id,
    status,
    limit,
    offset
  });
  const deliveries = result.deliveries.map((del) => ({
    ...del,
    payload: void 0,
    payloadPreview: JSON.parse(del.payload).type
  }));
  return c.json({
    deliveries,
    total: result.total,
    hasMore: result.hasMore
  });
});
function getEventDescription(event) {
  const descriptions = {
    "memory.remembered": "Triggered when a new memory is created",
    "memory.forgotten": "Triggered when a memory is deleted",
    "memory.updated": "Triggered when a memory is updated",
    "knowledge.ingested": "Triggered when a document is ingested into the knowledge bank",
    "knowledge.deleted": "Triggered when a knowledge source is deleted"
  };
  return descriptions[event] || event;
}
__name(getEventDescription, "getEventDescription");
var webhooks_default = webhookRoutes;

// src/routes/agent-tokens.ts
var agentTokenRoutes = new Hono2();
agentTokenRoutes.use("*", requireJwtAuth);
function validatePermissions(permissions) {
  if (!Array.isArray(permissions))
    return null;
  if (permissions.length === 0)
    return null;
  const validPermissions = ["read", "write"];
  for (const perm of permissions) {
    if (typeof perm !== "string")
      return null;
    if (!validPermissions.includes(perm))
      return null;
  }
  return permissions;
}
__name(validatePermissions, "validatePermissions");
function validateAllowedMemories(memories) {
  if (!Array.isArray(memories))
    return null;
  if (memories.length === 0)
    return null;
  for (const mem of memories) {
    if (typeof mem !== "string")
      return null;
    if (mem.length === 0)
      return null;
  }
  return memories;
}
__name(validateAllowedMemories, "validateAllowedMemories");
agentTokenRoutes.get("/", async (c) => {
  const userId = c.get("userId");
  if (!c.env?.DB) {
    return c.json({ error: "Database not available" }, 503);
  }
  if (!userId) {
    return c.json({ error: "User ID not found" }, 401);
  }
  const service = new AgentTokenService(c.env.DB);
  const activeOnly = c.req.query("active") === "true";
  const limit = parseInt(c.req.query("limit") || "100", 10);
  const offset = parseInt(c.req.query("offset") || "0", 10);
  const result = await service.list({ userId, activeOnly, limit, offset });
  return c.json({
    tokens: result.tokens,
    total: result.total,
    hasMore: result.hasMore
  });
});
agentTokenRoutes.get("/stats", async (c) => {
  const userId = c.get("userId");
  if (!c.env?.DB) {
    return c.json({ error: "Database not available" }, 503);
  }
  if (!userId) {
    return c.json({ error: "User ID not found" }, 401);
  }
  const service = new AgentTokenService(c.env.DB);
  const stats = await service.getStats(userId);
  return c.json({ stats });
});
agentTokenRoutes.post("/", async (c) => {
  const userId = c.get("userId");
  if (!c.env?.DB) {
    return c.json({ error: "Database not available" }, 503);
  }
  if (!userId) {
    return c.json({ error: "User ID not found" }, 401);
  }
  try {
    const body = await c.req.json();
    const {
      name,
      description,
      allowedMemories,
      permissions,
      expiresAt,
      tenantId
    } = body;
    if (!name || typeof name !== "string" || name.trim().length === 0) {
      return c.json({ error: "name is required" }, 400);
    }
    if (name.length > 100) {
      return c.json({ error: "name must be 100 characters or less" }, 400);
    }
    let validatedPermissions;
    if (permissions !== void 0) {
      const result = validatePermissions(permissions);
      if (!result) {
        return c.json({
          error: "permissions must be a non-empty array of valid permissions",
          validPermissions: ["read", "write"]
        }, 400);
      }
      validatedPermissions = result;
    }
    let validatedMemories;
    if (allowedMemories !== void 0) {
      const result = validateAllowedMemories(allowedMemories);
      if (!result) {
        return c.json({
          error: 'allowedMemories must be a non-empty array of memory IDs (or ["*"] for all)'
        }, 400);
      }
      validatedMemories = result;
    }
    if (expiresAt !== void 0) {
      if (typeof expiresAt !== "number" || expiresAt <= Date.now()) {
        return c.json({ error: "expiresAt must be a future timestamp" }, 400);
      }
    }
    const service = new AgentTokenService(c.env.DB);
    const options = {
      userId,
      tenantId,
      name: name.trim(),
      description: description?.trim(),
      allowedMemories: validatedMemories,
      permissions: validatedPermissions,
      expiresAt
    };
    const token = await service.create(options);
    return c.json({
      token,
      message: "Agent token created. Use this ID with your API key to authenticate MCP connections."
    }, 201);
  } catch (error) {
    console.error("Error creating agent token:", error);
    return c.json({ error: "Failed to create agent token" }, 500);
  }
});
agentTokenRoutes.get("/:id", async (c) => {
  const id = c.req.param("id");
  const userId = c.get("userId");
  if (!c.env?.DB) {
    return c.json({ error: "Database not available" }, 503);
  }
  if (!userId) {
    return c.json({ error: "User ID not found" }, 401);
  }
  const service = new AgentTokenService(c.env.DB);
  const token = await service.getByIdAndUser(id, userId);
  if (!token) {
    return c.json({ error: "Agent token not found" }, 404);
  }
  return c.json({ token });
});
agentTokenRoutes.patch("/:id", async (c) => {
  const id = c.req.param("id");
  const userId = c.get("userId");
  if (!c.env?.DB) {
    return c.json({ error: "Database not available" }, 503);
  }
  if (!userId) {
    return c.json({ error: "User ID not found" }, 401);
  }
  const service = new AgentTokenService(c.env.DB);
  const existing = await service.getByIdAndUser(id, userId);
  if (!existing) {
    return c.json({ error: "Agent token not found" }, 404);
  }
  try {
    const body = await c.req.json();
    const updates = {};
    if (body.name !== void 0) {
      if (typeof body.name !== "string" || body.name.trim().length === 0) {
        return c.json({ error: "name must be a non-empty string" }, 400);
      }
      if (body.name.length > 100) {
        return c.json({ error: "name must be 100 characters or less" }, 400);
      }
      updates.name = body.name.trim();
    }
    if (body.description !== void 0) {
      updates.description = body.description?.trim() || void 0;
    }
    if (body.permissions !== void 0) {
      const validatedPermissions = validatePermissions(body.permissions);
      if (!validatedPermissions) {
        return c.json({
          error: "permissions must be a non-empty array of valid permissions",
          validPermissions: ["read", "write"]
        }, 400);
      }
      updates.permissions = validatedPermissions;
    }
    if (body.allowedMemories !== void 0) {
      const validatedMemories = validateAllowedMemories(body.allowedMemories);
      if (!validatedMemories) {
        return c.json({
          error: "allowedMemories must be a non-empty array of memory IDs"
        }, 400);
      }
      updates.allowedMemories = validatedMemories;
    }
    if (body.isActive !== void 0) {
      updates.isActive = Boolean(body.isActive);
    }
    if (body.expiresAt !== void 0) {
      if (body.expiresAt === null) {
        updates.expiresAt = null;
      } else if (typeof body.expiresAt === "number" && body.expiresAt > Date.now()) {
        updates.expiresAt = body.expiresAt;
      } else {
        return c.json({ error: "expiresAt must be null or a future timestamp" }, 400);
      }
    }
    const token = await service.update(id, updates);
    return c.json({ token });
  } catch (error) {
    console.error("Error updating agent token:", error);
    return c.json({ error: "Failed to update agent token" }, 500);
  }
});
agentTokenRoutes.delete("/:id", async (c) => {
  const id = c.req.param("id");
  const userId = c.get("userId");
  if (!c.env?.DB) {
    return c.json({ error: "Database not available" }, 503);
  }
  if (!userId) {
    return c.json({ error: "User ID not found" }, 401);
  }
  const service = new AgentTokenService(c.env.DB);
  const existing = await service.getByIdAndUser(id, userId);
  if (!existing) {
    return c.json({ error: "Agent token not found" }, 404);
  }
  const deleted = await service.delete(id);
  if (!deleted) {
    return c.json({ error: "Failed to delete agent token" }, 500);
  }
  return c.json({ success: true });
});
agentTokenRoutes.post("/:id/toggle", async (c) => {
  const id = c.req.param("id");
  const userId = c.get("userId");
  if (!c.env?.DB) {
    return c.json({ error: "Database not available" }, 503);
  }
  if (!userId) {
    return c.json({ error: "User ID not found" }, 401);
  }
  const service = new AgentTokenService(c.env.DB);
  const existing = await service.getByIdAndUser(id, userId);
  if (!existing) {
    return c.json({ error: "Agent token not found" }, 404);
  }
  const token = await service.toggle(id);
  return c.json({
    token,
    message: token?.isActive ? "Agent token activated" : "Agent token deactivated"
  });
});
agentTokenRoutes.post("/:id/add-memory", async (c) => {
  const id = c.req.param("id");
  const userId = c.get("userId");
  if (!c.env?.DB) {
    return c.json({ error: "Database not available" }, 503);
  }
  if (!userId) {
    return c.json({ error: "User ID not found" }, 401);
  }
  const service = new AgentTokenService(c.env.DB);
  const existing = await service.getByIdAndUser(id, userId);
  if (!existing) {
    return c.json({ error: "Agent token not found" }, 404);
  }
  try {
    const body = await c.req.json();
    const { memoryId } = body;
    if (!memoryId || typeof memoryId !== "string") {
      return c.json({ error: "memoryId is required" }, 400);
    }
    const token = await service.addAllowedMemory(id, memoryId);
    return c.json({ token });
  } catch (error) {
    console.error("Error adding memory:", error);
    return c.json({ error: "Failed to add memory" }, 500);
  }
});
agentTokenRoutes.post("/:id/remove-memory", async (c) => {
  const id = c.req.param("id");
  const userId = c.get("userId");
  if (!c.env?.DB) {
    return c.json({ error: "Database not available" }, 503);
  }
  if (!userId) {
    return c.json({ error: "User ID not found" }, 401);
  }
  const service = new AgentTokenService(c.env.DB);
  const existing = await service.getByIdAndUser(id, userId);
  if (!existing) {
    return c.json({ error: "Agent token not found" }, 404);
  }
  try {
    const body = await c.req.json();
    const { memoryId } = body;
    if (!memoryId || typeof memoryId !== "string") {
      return c.json({ error: "memoryId is required" }, 400);
    }
    const token = await service.removeAllowedMemory(id, memoryId);
    if (!token) {
      return c.json({
        error: "Cannot remove memory - token must have at least one allowed memory"
      }, 400);
    }
    return c.json({ token });
  } catch (error) {
    console.error("Error removing memory:", error);
    return c.json({ error: "Failed to remove memory" }, 500);
  }
});
var agent_tokens_default = agentTokenRoutes;

// src/index.ts
var DEFAULT_DIMENSIONS = 1536;
var AUTH_ENABLED = true;
var managers = /* @__PURE__ */ new Map();
var namespaceConfig = /* @__PURE__ */ new Map();
function getManager(namespace, dimensions) {
  let manager = managers.get(namespace);
  if (!manager) {
    const dims = dimensions || namespaceConfig.get(namespace) || DEFAULT_DIMENSIONS;
    manager = new MemoryManager({
      dimensions: dims,
      textFields: ["content", "event", "fact", "context", "description"]
    });
    managers.set(namespace, manager);
    namespaceConfig.set(namespace, dims);
  }
  return manager;
}
__name(getManager, "getManager");
function createNamespaceLocal(name, dimensions) {
  if (managers.has(name)) {
    throw new Error(`Namespace "${name}" already exists`);
  }
  namespaceConfig.set(name, dimensions);
  const manager = new MemoryManager({
    dimensions,
    textFields: ["content", "event", "fact", "context", "description"]
  });
  managers.set(name, manager);
  return manager;
}
__name(createNamespaceLocal, "createNamespaceLocal");
var app = new Hono2();
app.use("*", logger());
app.use("*", cors());
app.use("*", prettyJSON());
if (AUTH_ENABLED) {
  app.use("/api/*", jwtAuth);
  app.use("/api/*", createAuthMiddleware({
    publicPaths: ["/api/v1/auth/register", "/api/v1/auth/login", "/api/v1/auth/refresh", "/api/v1/auth/validate-agent"]
  }));
  app.use("/api/*", extractTenant);
  app.use("/api/*", createRateLimitMiddleware({
    defaultLimit: 100,
    defaultWindow: 60
  }));
}
app.route("/api/v1/auth", auth_default);
app.route("/api/v1/tenants", tenants_default);
app.route("/api/v1/audit", audit_default);
app.route("/api/v1/knowledge", knowledge_default);
app.route("/api/v1/webhooks", webhooks_default);
app.route("/api/v1/agent-tokens", agent_tokens_default);
app.get("/", (c) => {
  const hasD1 = !!c.env?.DB;
  const hasJWT = !!c.env?.JWT_SECRET;
  const hasAI = !!c.env?.AI;
  return c.json({
    service: "minimemory",
    version: "0.4.0",
    status: "ok",
    storage: hasD1 ? "d1" : "memory",
    embeddings: hasAI ? "workers-ai" : "external",
    auth: {
      enabled: AUTH_ENABLED,
      jwt: hasJWT ? "configured" : "not configured",
      apiKeys: "supported",
      devKey: AUTH_ENABLED ? "mm_dev_key_12345" : void 0
    },
    endpoints: {
      // Auth
      "POST /api/v1/auth/register": "Create new account",
      "POST /api/v1/auth/login": "Login and get tokens",
      "POST /api/v1/auth/refresh": "Refresh access token",
      "POST /api/v1/auth/logout": "Logout and invalidate session",
      "GET /api/v1/auth/me": "Get current user profile",
      // Tenants
      "GET /api/v1/tenants": "List user tenants",
      "POST /api/v1/tenants": "Create new tenant",
      "GET /api/v1/tenants/:id": "Get tenant details",
      "PUT /api/v1/tenants/:id": "Update tenant",
      "DELETE /api/v1/tenants/:id": "Delete tenant",
      "GET /api/v1/tenants/:id/members": "List tenant members",
      "POST /api/v1/tenants/:id/members": "Invite member",
      // Memory
      "POST /api/v1/remember": "Store a memory (auto-generates embedding)",
      "POST /api/v1/recall": "Search for memories (query auto-generates embedding)",
      "DELETE /api/v1/forget/:id": "Delete a memory",
      "POST /api/v1/forget": "Delete memories by filter",
      "GET /api/v1/memory/:id": "Get a specific memory",
      "PATCH /api/v1/memory/:id": "Update a memory",
      "GET /api/v1/stats": "Get memory statistics",
      "POST /api/v1/cleanup": "Clean up expired memories",
      "POST /api/v1/decay": "Apply importance decay",
      "POST /api/v1/export": "Export all memories",
      "POST /api/v1/import": "Import memories",
      "DELETE /api/v1/clear": "Clear all memories",
      // Embeddings
      "POST /api/v1/embed": "Generate embeddings (EmbeddingGemma)",
      "GET /api/v1/embed/info": "Get embedding service info",
      // Audit
      "GET /api/v1/audit": "Query audit logs",
      "GET /api/v1/audit/:id": "Get audit entry by ID",
      "GET /api/v1/audit/resource/:type/:id": "Get resource history",
      "GET /api/v1/audit/user/:id": "Get user activity",
      "GET /api/v1/audit/failures": "Get failed operations",
      "GET /api/v1/audit/stats": "Get audit statistics",
      "POST /api/v1/audit/cleanup": "Clean up old audit logs",
      // Knowledge Bank (RAG)
      "POST /api/v1/knowledge/ingest": "Ingest document into knowledge bank",
      "GET /api/v1/knowledge/sources": "List knowledge sources",
      "GET /api/v1/knowledge/sources/:id": "Get source details",
      "DELETE /api/v1/knowledge/sources/:id": "Delete source and chunks",
      "GET /api/v1/knowledge/sources/:id/chunks": "Get source chunks",
      "GET /api/v1/knowledge/stats": "Get knowledge bank statistics",
      "POST /api/v1/knowledge/chunk-preview": "Preview document chunking",
      // Webhooks
      "GET /api/v1/webhooks/events": "List available webhook events",
      "GET /api/v1/webhooks": "List webhooks",
      "POST /api/v1/webhooks": "Create webhook",
      "GET /api/v1/webhooks/:id": "Get webhook details",
      "PUT /api/v1/webhooks/:id": "Update webhook",
      "DELETE /api/v1/webhooks/:id": "Delete webhook",
      "POST /api/v1/webhooks/:id/test": "Test webhook",
      "POST /api/v1/webhooks/:id/rotate-secret": "Rotate webhook secret",
      "GET /api/v1/webhooks/:id/deliveries": "Get delivery history",
      // Agent Tokens (MCP access control)
      "GET /api/v1/agent-tokens": "List agent tokens",
      "POST /api/v1/agent-tokens": "Create agent token",
      "GET /api/v1/agent-tokens/:id": "Get agent token",
      "PATCH /api/v1/agent-tokens/:id": "Update agent token",
      "DELETE /api/v1/agent-tokens/:id": "Delete agent token",
      "POST /api/v1/agent-tokens/:id/toggle": "Toggle token active status",
      "POST /api/v1/agent-tokens/:id/add-memory": "Add memory to allowed list",
      "POST /api/v1/agent-tokens/:id/remove-memory": "Remove memory from allowed list",
      "POST /api/v1/auth/validate-agent": "Validate API key + agent token (for MCP)"
    }
  });
});
app.get("/health", (c) => {
  return c.json({
    status: "healthy",
    timestamp: (/* @__PURE__ */ new Date()).toISOString(),
    storage: c.env?.DB ? "d1" : "memory"
  });
});
app.route("/api/v1", createMemoryRoutes(getManager));
app.get("/api/v1/namespaces", async (c) => {
  if (c.env?.DB) {
    const storage = new D1Storage(c.env.DB);
    const namespaces2 = await storage.listNamespaces();
    return c.json({
      success: true,
      namespaces: namespaces2.map((ns) => ({ name: ns.name, dimensions: ns.dimensions })),
      count: namespaces2.length,
      storage: "d1"
    });
  }
  const namespaces = Array.from(managers.keys()).map((name) => ({
    name,
    dimensions: namespaceConfig.get(name) || DEFAULT_DIMENSIONS
  }));
  return c.json({
    success: true,
    namespaces,
    count: namespaces.length,
    storage: "memory"
  });
});
app.post("/api/v1/namespaces", async (c) => {
  try {
    const body = await c.req.json();
    const { name, dimensions } = body;
    if (!name || typeof name !== "string") {
      return c.json({ error: "name is required" }, 400);
    }
    if (!dimensions || typeof dimensions !== "number" || dimensions < 1) {
      return c.json({ error: "dimensions must be a positive number" }, 400);
    }
    if (c.env?.DB) {
      const storage = new D1Storage(c.env.DB);
      const existing = await storage.getNamespace(name);
      if (existing) {
        return c.json({ error: `Namespace "${name}" already exists` }, 400);
      }
      await storage.createNamespace(name, dimensions);
      namespaceConfig.set(name, dimensions);
    } else {
      createNamespaceLocal(name, dimensions);
    }
    return c.json({
      success: true,
      namespace: { name, dimensions },
      message: `Namespace "${name}" created with ${dimensions} dimensions`,
      storage: c.env?.DB ? "d1" : "memory"
    });
  } catch (error) {
    return c.json({
      error: error instanceof Error ? error.message : "Unknown error"
    }, 400);
  }
});
app.delete("/api/v1/namespaces/:name", async (c) => {
  const name = c.req.param("name");
  let deleted = false;
  if (c.env?.DB) {
    const storage = new D1Storage(c.env.DB);
    deleted = await storage.deleteNamespace(name);
  }
  managers.delete(name);
  namespaceConfig.delete(name);
  return c.json({
    success: deleted || managers.has(name) === false,
    message: deleted ? `Namespace "${name}" deleted` : `Namespace "${name}" not found`
  });
});
app.onError((err, c) => {
  console.error("Error:", err);
  return c.json({
    error: err.message || "Internal server error"
  }, 500);
});
app.notFound((c) => {
  return c.json({
    error: "Not found",
    path: c.req.path
  }, 404);
});
var src_default = app;

// node_modules/wrangler/templates/middleware/middleware-ensure-req-body-drained.ts
var drainBody = /* @__PURE__ */ __name(async (request, env, _ctx, middlewareCtx) => {
  try {
    return await middlewareCtx.next(request, env);
  } finally {
    try {
      if (request.body !== null && !request.bodyUsed) {
        const reader = request.body.getReader();
        while (!(await reader.read()).done) {
        }
      }
    } catch (e) {
      console.error("Failed to drain the unused request body.", e);
    }
  }
}, "drainBody");
var middleware_ensure_req_body_drained_default = drainBody;

// node_modules/wrangler/templates/middleware/middleware-miniflare3-json-error.ts
function reduceError(e) {
  return {
    name: e?.name,
    message: e?.message ?? String(e),
    stack: e?.stack,
    cause: e?.cause === void 0 ? void 0 : reduceError(e.cause)
  };
}
__name(reduceError, "reduceError");
var jsonError = /* @__PURE__ */ __name(async (request, env, _ctx, middlewareCtx) => {
  try {
    return await middlewareCtx.next(request, env);
  } catch (e) {
    const error = reduceError(e);
    return Response.json(error, {
      status: 500,
      headers: { "MF-Experimental-Error-Stack": "true" }
    });
  }
}, "jsonError");
var middleware_miniflare3_json_error_default = jsonError;

// .wrangler/tmp/bundle-fCESHO/middleware-insertion-facade.js
var __INTERNAL_WRANGLER_MIDDLEWARE__ = [
  middleware_ensure_req_body_drained_default,
  middleware_miniflare3_json_error_default
];
var middleware_insertion_facade_default = src_default;

// node_modules/wrangler/templates/middleware/common.ts
var __facade_middleware__ = [];
function __facade_register__(...args) {
  __facade_middleware__.push(...args.flat());
}
__name(__facade_register__, "__facade_register__");
function __facade_invokeChain__(request, env, ctx, dispatch, middlewareChain) {
  const [head, ...tail] = middlewareChain;
  const middlewareCtx = {
    dispatch,
    next(newRequest, newEnv) {
      return __facade_invokeChain__(newRequest, newEnv, ctx, dispatch, tail);
    }
  };
  return head(request, env, ctx, middlewareCtx);
}
__name(__facade_invokeChain__, "__facade_invokeChain__");
function __facade_invoke__(request, env, ctx, dispatch, finalMiddleware) {
  return __facade_invokeChain__(request, env, ctx, dispatch, [
    ...__facade_middleware__,
    finalMiddleware
  ]);
}
__name(__facade_invoke__, "__facade_invoke__");

// .wrangler/tmp/bundle-fCESHO/middleware-loader.entry.ts
var __Facade_ScheduledController__ = class {
  constructor(scheduledTime, cron, noRetry) {
    this.scheduledTime = scheduledTime;
    this.cron = cron;
    this.#noRetry = noRetry;
  }
  #noRetry;
  noRetry() {
    if (!(this instanceof __Facade_ScheduledController__)) {
      throw new TypeError("Illegal invocation");
    }
    this.#noRetry();
  }
};
__name(__Facade_ScheduledController__, "__Facade_ScheduledController__");
function wrapExportedHandler(worker) {
  if (__INTERNAL_WRANGLER_MIDDLEWARE__ === void 0 || __INTERNAL_WRANGLER_MIDDLEWARE__.length === 0) {
    return worker;
  }
  for (const middleware of __INTERNAL_WRANGLER_MIDDLEWARE__) {
    __facade_register__(middleware);
  }
  const fetchDispatcher = /* @__PURE__ */ __name(function(request, env, ctx) {
    if (worker.fetch === void 0) {
      throw new Error("Handler does not export a fetch() function.");
    }
    return worker.fetch(request, env, ctx);
  }, "fetchDispatcher");
  return {
    ...worker,
    fetch(request, env, ctx) {
      const dispatcher = /* @__PURE__ */ __name(function(type, init) {
        if (type === "scheduled" && worker.scheduled !== void 0) {
          const controller = new __Facade_ScheduledController__(
            Date.now(),
            init.cron ?? "",
            () => {
            }
          );
          return worker.scheduled(controller, env, ctx);
        }
      }, "dispatcher");
      return __facade_invoke__(request, env, ctx, dispatcher, fetchDispatcher);
    }
  };
}
__name(wrapExportedHandler, "wrapExportedHandler");
function wrapWorkerEntrypoint(klass) {
  if (__INTERNAL_WRANGLER_MIDDLEWARE__ === void 0 || __INTERNAL_WRANGLER_MIDDLEWARE__.length === 0) {
    return klass;
  }
  for (const middleware of __INTERNAL_WRANGLER_MIDDLEWARE__) {
    __facade_register__(middleware);
  }
  return class extends klass {
    #fetchDispatcher = (request, env, ctx) => {
      this.env = env;
      this.ctx = ctx;
      if (super.fetch === void 0) {
        throw new Error("Entrypoint class does not define a fetch() function.");
      }
      return super.fetch(request);
    };
    #dispatcher = (type, init) => {
      if (type === "scheduled" && super.scheduled !== void 0) {
        const controller = new __Facade_ScheduledController__(
          Date.now(),
          init.cron ?? "",
          () => {
          }
        );
        return super.scheduled(controller);
      }
    };
    fetch(request) {
      return __facade_invoke__(
        request,
        this.env,
        this.ctx,
        this.#dispatcher,
        this.#fetchDispatcher
      );
    }
  };
}
__name(wrapWorkerEntrypoint, "wrapWorkerEntrypoint");
var WRAPPED_ENTRY;
if (typeof middleware_insertion_facade_default === "object") {
  WRAPPED_ENTRY = wrapExportedHandler(middleware_insertion_facade_default);
} else if (typeof middleware_insertion_facade_default === "function") {
  WRAPPED_ENTRY = wrapWorkerEntrypoint(middleware_insertion_facade_default);
}
var middleware_loader_entry_default = WRAPPED_ENTRY;
export {
  ApiKeyStore,
  D1Storage,
  __INTERNAL_WRANGLER_MIDDLEWARE__,
  app,
  middleware_loader_entry_default as default,
  defaultKeyStore,
  getManager,
  managers
};
//# sourceMappingURL=index.js.map
