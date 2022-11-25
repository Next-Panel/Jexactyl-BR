@extends('layouts.admin')

@section('title')
Servidor  — {{ $server->name }}: Gerenciar
@endsection

@section('content-header')
    <h1>{{ $server->name }}<small>Ações adicionais para controlar este servidor.</small></h1>
    <ol class="breadcrumb">
        <li><a href="{{ route('admin.index') }}">Admin</a></li>
        <li><a href="{{ route('admin.servers') }}">Servidores</a></li>
        <li><a href="{{ route('admin.servers.view', $server->id) }}">{{ $server->name }}</a></li>
        <li class="active">Gerenciar</li>
    </ol>
@endsection

@section('content')
    @include('admin.servers.partials.navigation')
    <div class="row">
        <div class="col-sm-4">
            <div class="box box-danger">
                <div class="box-header with-border">
                    <h3 class="box-title">Reinstalar servidor</h3>
                </div>
                <div class="box-body">
                    <p>Isto reinstalará o servidor com os scripts de serviço designados.
                         <strong>Perigo!</strong> 
                         Isto poderia sobrescrever os dados do servidor.</p>
                </div>
                <div class="box-footer">
                    @if($server->isInstalled())
                        <form action="{{ route('admin.servers.view.manage.reinstall', $server->id) }}" method="POST">
                            {!! csrf_field() !!}
                            <button type="submit" class="btn btn-danger">Reinstalar servidor</button>
                        </form>
                    @else
                        <button class="btn btn-danger disabled">O servidor deve ser instalado corretamente para reinstalar</button>
                    @endif
                </div>
            </div>
        </div>
        <div class="col-sm-4">
            <div class="box box-primary">
                <div class="box-header with-border">
                    <h3 class="box-title">Status de instalação</h3>
                </div>
                <div class="box-body">
                    <p>Se você precisar mudar o status de instalação de desinstalado para instalado, ou vice-versa, você pode fazê-lo com o botão abaixo.</p>
                </div>
                <div class="box-footer">
                    <form action="{{ route('admin.servers.view.manage.toggle', $server->id) }}" method="POST">
                        {!! csrf_field() !!}
                        <button type="submit" class="btn btn-primary">Alternar o status de instalação</button>
                    </form>
                </div>
            </div>
        </div>

        @if(! $server->isSuspended())
            <div class="col-sm-4">
                <div class="box box-warning">
                    <div class="box-header with-border">
                        <h3 class="box-title">Suspender Servidor</h3>
                    </div>
                    <div class="box-body">
                        <p>Isto suspenderá o servidor, interromperá qualquer processo em execução e bloqueará imediatamente o usuário de poder acessar seus arquivos ou de outra forma gerenciar o servidor através do painel ou API.</p>
                    </div>
                    <div class="box-footer">
                        <form action="{{ route('admin.servers.view.manage.suspension', $server->id) }}" method="POST">
                            {!! csrf_field() !!}
                            <input type="hidden" name="action" value="suspend" />
                            <button type="submit" class="btn btn-warning @if(! is_null($server->transfer)) disabled @endif">Servidor Suspenso</button>
                        </form>
                    </div>
                </div>
            </div>
        @else
            <div class="col-sm-4">
                <div class="box box-success">
                    <div class="box-header with-border">
                        <h3 class="box-title">Desbloquear servidor</h3>
                    </div>
                    <div class="box-body">
                        <p>Isto irá desuspenderá o servidor e restaurar o acesso normal do usuário.</p>
                    </div>
                    <div class="box-footer">
                        <form action="{{ route('admin.servers.view.manage.suspension', $server->id) }}" method="POST">
                            {!! csrf_field() !!}
                            <input type="hidden" name="action" value="unsuspend" />
                            <button type="submit" class="btn btn-success">Desbloquear servidor</button>
                        </form>
                    </div>
                </div>
            </div>
        @endif

        @if(is_null($server->transfer))
            <div class="col-sm-4">
                <div class="box box-success">
                    <div class="box-header with-border">
                        <h3 class="box-title">Transferir Servidor</h3>
                    </div>
                    <div class="box-body">
                        <p>
                        Transfira este servidor para outro Node conectado a este painel.
                            <strong>Atenção!</strong> Esta funcionalidade não foi totalmente testada e pode ter bugs.
                        </p>
                    </div>

                    <div class="box-footer">
                        @if($canTransfer)
                            <button class="btn btn-success" data-toggle="modal" data-target="#transferServerModal">Transfer Server</button>
                        @else
                            <button class="btn btn-success disabled">Transferir Servidor</button>
                            <p style="padding-top: 1rem;">A transferência de um servidor requer mais de um Node a ser configurado em seu painel.</p>
                        @endif
                    </div>
                </div>
            </div>
        @else
            <div class="col-sm-4">
                <div class="box box-success">
                    <div class="box-header with-border">
                        <h3 class="box-title">Transferir Servidor</h3>
                    </div>
                    <div class="box-body">
                        <p>
                        Este servidor está sendo transferido atualmente para outro Node.
                        A transferência foi iniciada em <strong>{{ $server->transfer->created_at }}</strong>
                        </p>
                    </div>

                    <div class="box-footer">
                        <button class="btn btn-success disabled">Transferir Servidor</button>
                    </div>
                </div>
            </div>
        @endif
    </div>

    <div class="modal fade" id="transferServerModal" tabindex="-1" role="dialog">
        <div class="modal-dialog" role="document">
            <div class="modal-content">
                <form action="{{ route('admin.servers.view.manage.transfer', $server->id) }}" method="POST">
                    <div class="modal-header">
                        <button type="button" class="close" data-dismiss="modal" aria-label="Close"><span aria-hidden="true">&times;</span></button>
                        <h4 class="modal-title">Transferir Servidor</h4>
                    </div>

                    <div class="modal-body">
                        <div class="row">
                            <div class="form-group col-md-12">
                                <label for="pNodeId">Node</label>
                                <select name="node_id" id="pNodeId" class="form-control">
                                    @foreach($locations as $location)
                                        <optgroup label="{{ $location->long }} ({{ $location->short }})">
                                            @foreach($location->nodes as $node)

                                                @if($node->id != $server->node_id)
                                                    <option value="{{ $node->id }}"
                                                            @if($location->id === old('location_id')) selected @endif
                                                    >{{ $node->name }}</option>
                                                @endif

                                            @endforeach
                                        </optgroup>
                                    @endforeach
                                </select>
                                <p class="small text-muted no-margin">O Node para o qual este servidor será transferido.</p>
                            </div>

                            <div class="form-group col-md-12">
                                <label for="pAllocation">Alocação padrão</label>
                                <select name="allocation_id" id="pAllocation" class="form-control"></select>
                                <p class="small text-muted no-margin">A principal alocação que será atribuída a este servidor.</p>
                            </div>

                            <div class="form-group col-md-12">
                                <label for="pAllocationAdditional">Alocação(ões) adicional(is)</label>
                                <select name="allocation_additional[]" id="pAllocationAdditional" class="form-control" multiple></select>
                                <p class="small text-muted no-margin">Alocações adicionais a serem atribuídas a este servidor na criação.</p>
                            </div>
                        </div>
                    </div>

                    <div class="modal-footer">
                        {!! csrf_field() !!}
                        <button type="button" class="btn btn-default btn-sm pull-left" data-dismiss="modal">Cancelar</button>
                        <button type="submit" class="btn btn-success btn-sm">Confirme</button>
                    </div>
                </form>
            </div>
        </div>
    </div>
@endsection

@section('footer-scripts')
    @parent
    {!! Theme::js('vendor/lodash/lodash.js') !!}

    @if($canTransfer)
        {!! Theme::js('js/admin/server/transfer.js') !!}
    @endif
@endsection
