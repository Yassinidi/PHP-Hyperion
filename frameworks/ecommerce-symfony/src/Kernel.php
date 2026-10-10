<?php
declare(strict_types=1);

namespace App;

use Symfony\Component\HttpKernel\HttpKernelInterface;
use Symfony\Component\HttpFoundation\Request;
use Symfony\Component\HttpFoundation\Response;
use Symfony\Component\HttpFoundation\JsonResponse;
use Symfony\Component\Routing\RouteCollection;
use Symfony\Component\Routing\Route;
use Symfony\Component\Routing\RequestContext;
use Symfony\Component\Routing\Matcher\UrlMatcher;
use Symfony\Component\Routing\Exception\ResourceNotFoundException;
use Symfony\Component\Routing\Exception\MethodNotAllowedException;
use App\Controller\StoreController;
use App\Controller\AdminController;

class Kernel implements HttpKernelInterface {
    private string $environment;
    private bool $debug;
    private RouteCollection $routes;
    private StoreController $storeController;
    private AdminController $adminController;

    public function __construct(string $environment = 'prod', bool $debug = false) {
        $this->environment = $environment;
        $this->debug = $debug;
        $this->storeController = new StoreController();
        $this->adminController = new AdminController();
        $this->routes = new RouteCollection();
        $this->configureRoutes();
    }

    public function boot(): void {
        Database::get();
    }

    private function configureRoutes(): void {
        $this->routes->add('home', new Route('/', [
            '_controller' => function(Request $r) { return $this->storeController->home($r); }
        ], [], [], '', [], ['GET']));

        $this->routes->add('admin', new Route('/admin', [
            '_controller' => function(Request $r) { return $this->adminController->admin(); }
        ], [], [], '', [], ['GET']));

        $this->routes->add('api_banners', new Route('/api/banners', [
            '_controller' => function() { return $this->storeController->banners(); }
        ], [], [], '', [], ['GET']));

        $this->routes->add('api_categories', new Route('/api/categories', [
            '_controller' => function() { return $this->storeController->categories(); }
        ], [], [], '', [], ['GET']));

        $this->routes->add('api_products', new Route('/api/products', [
            '_controller' => function(Request $r) { return $this->storeController->products($r); }
        ], [], [], '', [], ['GET']));

        $this->routes->add('api_product_detail', new Route('/api/products/{id}', [
            '_controller' => function(Request $r, array $p) { return $this->storeController->productDetail((int)$p['id']); }
        ], ['id' => '\d+'], [], '', [], ['GET']));

        $this->routes->add('api_checkout', new Route('/api/cart/checkout', [
            '_controller' => function(Request $r) { return $this->storeController->checkout($r); }
        ], [], [], '', [], ['POST']));

        $this->routes->add('api_admin_dashboard', new Route('/api/admin/dashboard', [
            '_controller' => function() { return $this->adminController->dashboard(); }
        ], [], [], '', [], ['GET']));

        $this->routes->add('api_admin_users', new Route('/api/admin/users', [
            '_controller' => function() { return $this->adminController->users(); }
        ], [], [], '', [], ['GET']));

        $this->routes->add('api_auth_login', new Route('/api/auth/login', [
            '_controller' => function(Request $r) { return $this->adminController->login($r); }
        ], [], [], '', [], ['POST']));

        $this->routes->add('health', new Route('/health', [
            '_controller' => function() {
                return new JsonResponse(['status' => 'ok', 'project' => 'ecommerce-symfony', 'timestamp' => microtime(true)]);
            }
        ]));
    }

    public function handle(Request $request, int $type = self::MAIN_REQUEST, bool $catch = true): Response {
        $context = new RequestContext();
        $context->fromRequest($request);
        $matcher = new UrlMatcher($this->routes, $context);

        try {
            $parameters = $matcher->match($request->getPathInfo());
            $controller = $parameters['_controller'];
            if (is_callable($controller)) {
                return $controller($request, $parameters);
            }
            return new JsonResponse(['error' => 'Invalid controller action'], 500);
        } catch (ResourceNotFoundException $e) {
            return new JsonResponse(['error' => 'Not Found', 'path' => $request->getPathInfo()], 404);
        } catch (MethodNotAllowedException $e) {
            return new JsonResponse(['error' => 'Method Not Allowed'], 405);
        } catch (\Throwable $e) {
            return new JsonResponse(['error' => $e->getMessage()], 500);
        }
    }

    public function terminate(Request $request, Response $response): void {
        // Request termination lifecycle
    }

    public function getContainer(): object {
        return new class {
            public function has(string $id): bool { return false; }
            public function get(string $id): mixed { return null; }
        };
    }
}
