import http from 'k6/http';
import { check, group, sleep } from 'k6';
import { Trend, Rate } from 'k6/metrics';

// Custom metrics per route
const homeDuration = new Trend('duration_home', true);
const aboutDuration = new Trend('duration_about', true);
const blogDuration = new Trend('duration_blog', true);
const exampleDuration = new Trend('duration_example', true);
const upDuration = new Trend('duration_up', true);
const loginDuration = new Trend('duration_login', true);
const adminDuration = new Trend('duration_admin', true);

const successRate = new Rate('successful_requests');

export const options = {
    scenarios: {
        max_concurrency_stress: {
            executor: 'ramping-vus',
            startVUs: 10,
            stages: [
                { duration: '3s', target: 50 },   // Warmup ramp to 50 VUs
                { duration: '10s', target: 100 },  // Peak load: 100 concurrent VUs!
                { duration: '5s', target: 100 },   // Sustained maximum saturation
                { duration: '2s', target: 0 },     // Graceful ramp-down
            ],
            gracefulRampDown: '1s',
        },
    },
    thresholds: {
        'http_req_failed': ['rate<0.01'], // <1% errors allowed
        'successful_requests': ['rate>0.99'],
    },
};

const BASE_URL = __ENV.TARGET_URL || 'http://127.0.0.1:8000';

const ROUTES = [
    { name: 'home', path: '/', trend: homeDuration, expectedStatus: 200 },
    { name: 'about', path: '/about', trend: aboutDuration, expectedStatus: 200 },
    { name: 'blog', path: '/blog', trend: blogDuration, expectedStatus: 200 },
    { name: 'example', path: '/example-page', trend: exampleDuration, expectedStatus: 200 },
    { name: 'up', path: '/up', trend: upDuration, expectedStatus: 200 },
    { name: 'login', path: '/auth/login', trend: loginDuration, expectedStatus: 200 },
    { name: 'admin', path: '/admin', trend: adminDuration, expectedStatus: 302 },
];

export default function () {
    // Round-robin or random distribution across all routes
    const routeIndex = Math.floor(Math.random() * ROUTES.length);
    const route = ROUTES[routeIndex];

    const params = {
        headers: {
            'User-Agent': 'k6-Hyperion-LoadTest/1.0',
            'Accept': 'text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8',
            'Connection': 'keep-alive',
        },
        tags: { route: route.name },
        redirects: 0, // Don't auto-follow to test exact status codes
    };

    const res = http.get(`${BASE_URL}${route.path}`, params);

    route.trend.add(res.timings.duration);

    const isOk = (res.status === route.expectedStatus);
    successRate.add(isOk);

    check(res, {
        'expected status code': (r) => r.status === route.expectedStatus,
    });
}
