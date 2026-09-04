import http from 'k6/http';
import { check } from 'k6';

export const options = {
    scenarios: {
        benchmark: {
            executor: 'constant-vus',
            vus: __ENV.VUS ? parseInt(__ENV.VUS) : 20,
            duration: __ENV.DURATION || '10s',
        },
    },
};

export default function () {
    const targetUrl = __ENV.TARGET_URL;
    const res = http.get(targetUrl, {
        timeout: '10s',
        headers: {
            'Accept': 'text/html,application/json',
            'Connection': 'keep-alive',
        }
    });

    check(res, {
        'status is 200': (r) => r.status === 200,
    });
}
