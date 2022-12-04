@php
    /** @var \Pterodactyl\Models\Server $server */
    $router = app('router');
@endphp
<div class="row">
    <div class="col-xs-12">
        <div class="nav-tabs-custom nav-tabs-floating">
            <ul class="nav nav-tabs">
                <li class="{{ $router->currentRouteNamed('admin.servers.view') ? 'active' : '' }}">
                    <a href="{{ route('admin.servers.view', $server->id) }}">Sobre</a></li>
                @if($server->isInstalled())
                    <li class="{{ $router->currentRouteNamed('admin.servers.view.details') ? 'active' : '' }}">
                        <a href="{{ route('admin.servers.view.details', $server->id) }}">Detalhes</a>
                    </li>
                    <li class="{{ $router->currentRouteNamed('admin.servers.view.build') ? 'active' : '' }}">
                        <a href="{{ route('admin.servers.view.build', $server->id) }}">Configuração de Build</a>
                    </li>
                    <li class="{{ $router->currentRouteNamed('admin.servers.view.startup') ? 'active' : '' }}">
                        <a href="{{ route('admin.servers.view.startup', $server->id) }}">Iniciação</a>
                    </li>
                    <li class="{{ $router->currentRouteNamed('admin.servers.view.database') ? 'active' : '' }}">
                        <a href="{{ route('admin.servers.view.database', $server->id) }}">Database</a>
                    </li>
                    <li class="{{ $router->currentRouteNamed('admin.servers.view.mounts') ? 'active' : '' }}">
                        <a href="{{ route('admin.servers.view.mounts', $server->id) }}">Montagem</a>
                    </li>
                @endif
                <li class="{{ $router->currentRouteNamed('admin.servers.view.manage') ? 'active' : '' }}">
                    <a href="{{ route('admin.servers.view.manage', $server->id) }}">Gerenciar</a>
                </li>
                <li class="tab-danger {{ $router->currentRouteNamed('admin.servers.view.delete') ? 'active' : '' }}">
                    <a href="{{ route('admin.servers.view.delete', $server->id) }}">Deletar</a>
                </li>
                <li class="tab-success">
                    <a href="/server/{{ $server->uuidShort }}" target="_blank"><i class="fa fa-external-link"></i></a>
                </li>
            </ul>
        </div>
    </div>
</div>
