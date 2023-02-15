@extends('layouts.admin')

@section('title')
    Host de database
@endsection

@section('content-header')
    <h1>Host de Database<small>Hospedagem de database nos quais os servidores podem ter bancos de dados criados.</small></h1>
    <ol class="breadcrumb">
        <li><a href="{{ route('admin.index') }}">Administrador</a></li>
        <li class="active">Lista dos Database</li>
    </ol>
@endsection

@section('content')
<div class="row">
    <div class="col-xs-12">
        <div class="box box-primary">
            <div class="box-header with-border">
                <h3 class="box-title">Lista dos Database</h3>
                <div class="box-tools">
                    <button class="btn btn-sm btn-primary" data-toggle="modal" data-target="#newHostModal">Criar Novo</button>
                </div>
            </div>
            <div class="box-body table-responsive no-padding">
                <table class="table table-hover">
                    <tbody>
                        <tr>
                            <th>ID</th>
                            <th>Nome</th>
                            <th>Hospedado</th>
                            <th>Porta</th>
                            <th>Usu&aacute;rio</th>
                            <th class="text-center">Database</th>
                            <th class="text-center">Node</th>
                        </tr>
                        @foreach ($hosts as $host)
                            <tr>
                                <td><code>{{ $host->id }}</code></td>
                                <td><a href="{{ route('admin.databases.view', $host->id) }}">{{ $host->name }}</a></td>
                                <td><code>{{ $host->host }}</code></td>
                                <td><code>{{ $host->port }}</code></td>
                                <td>{{ $host->username }}</td>
                                <td class="text-center">{{ $host->databases_count }}</td>
                                <td class="text-center">
                                    @if(! is_null($host->node))
                                        <a href="{{ route('admin.nodes.view', $host->node->id) }}">{{ $host->node->name }}</a>
                                    @else
                                        <span class="label label-default">Nada</span>
                                    @endif
                                </td>
                            </tr>
                        @endforeach
                    </tbody>
                </table>
            </div>
        </div>
    </div>
</div>
<div class="modal fade" id="newHostModal" tabindex="-1" role="dialog">
    <div class="modal-dialog" role="document">
        <div class="modal-content">
            <form action="{{ route('admin.databases') }}" method="POST">
                <div class="modal-header">
                    <button type="button" class="close" data-dismiss="modal" aria-label="Close"><span aria-hidden="true">&times;</span></button>
                    <h4 class="modal-title">Criar Novo Database</h4>
                </div>
                <div class="modal-body">
                    <div class="form-group">
                        <label for="pName" class="form-label">Nome</label>
                        <input type="text" name="name" id="pName" class="form-control" />
                        <p class="text-muted small">Um pequeno identificador utilizado para distinguir este local de outros. Deve ter entre 1 e 60 caracteres, por exemplo, <code>br.painel.db</code>.</p>
                    </div>
                    <div class="row">
                        <div class="col-md-6">
                            <label for="pHost" class="form-label">Hospedagem</label>
                            <input type="text" name="host" id="pHost" class="form-control" />
                            <p class="text-muted small">O endereço IP ou FQDN que deve ser usado quando se tenta conectar a este host MySQL <em> do painel</em> para adicionar novos databases.</p>
                        </div>
                        <div class="col-md-6">
                            <label for="pPort" class="form-label">Porta</label>
                            <input type="text" name="port" id="pPort" class="form-control" value="3306"/>
                            <p class="text-muted small">A porta em que o MySQL está rodando para este host.</p>
                        </div>
                    </div>
                    <div class="row">
                        <div class="col-md-6">
                            <label for="pUsername" class="form-label">Usu&aacute;rio</label>
                            <input type="text" name="username" id="pUsername" class="form-control" />
                            <p class="text-muted small">O nome de usuário de uma conta que tem permissões suficientes para criar novos usuários e bancos de dados sobre o sistema.</p>
                        </div>
                        <div class="col-md-6">
                            <label for="pPassword" class="form-label">Senha</label>
                            <input type="password" name="password" id="pPassword" class="form-control" />
                            <p class="text-muted small">A senha para a conta definida.</p>
                        </div>
                    </div>
                    <div class="form-group">
                        <label for="pNodeId" class="form-label">Node linkado</label>
                        <select name="node_id" id="pNodeId" class="form-control">
                            <option value="">Nada</option>
                            @foreach($locations as $location)
                                <optgroup label="{{ $location->short }}">
                                    @foreach($location->nodes as $node)
                                        <option value="{{ $node->id }}">{{ $node->name }}</option>
                                    @endforeach
                                </optgroup>
                            @endforeach
                        </select>
                        <p class="text-muted small">Esta configuração não faz nada além do padrão para este host de Database ao adicionar um Database a um servidor no Node selecionado.</p>
                    </div>
                </div>
                <div class="modal-footer">
                    <p class="text-danger small text-left">A conta definida para esse host de Database <strong>deve</strong> ter a permissão com <code>OPÇÃO DE CONCESSÃO</code>. Se a conta definida não tiver essa permissão, as solicitações para criar bancos de dados <em>falharão</em>. <strong>Não use os mesmos detalhes da conta para o MySQL que você definiu para este painel.</strong></p>
                    {!! csrf_field() !!}
                    <button type="button" class="btn btn-default btn-sm pull-left" data-dismiss="modal">Cancelar</button>
                    <button type="submit" class="btn btn-success btn-sm">Criar</button>
                </div>
            </form>
        </div>
    </div>
</div>
@endsection

@section('footer-scripts')
    @parent
    <script>
        $('#pNodeId').select2();
    </script>
@endsection
