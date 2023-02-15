@extends('layouts.admin')

@section('title')
    {{ $node->name }}: Configuração
@endsection

@section('content-header')
    <h1>{{ $node->name }}<small>Seu arquivo de configuração do daemon.</small></h1>
    <ol class="breadcrumb">
        <li><a href="{{ route('admin.index') }}">Administrador</a></li>
        <li><a href="{{ route('admin.nodes') }}">Nodes</a></li>
        <li><a href="{{ route('admin.nodes.view', $node->id) }}">{{ $node->name }}</a></li>
        <li class="active">Configuração</li>
    </ol>
@endsection

@section('content')
<div class="row">
    <div class="col-xs-12">
        <div class="nav-tabs-custom nav-tabs-floating">
            <ul class="nav nav-tabs">
                <li><a href="{{ route('admin.nodes.view', $node->id) }}">Sobre</a></li>
                <li><a href="{{ route('admin.nodes.view.settings', $node->id) }}">Definições</a></li>
                <li class="active"><a href="{{ route('admin.nodes.view.configuration', $node->id) }}">Configuração</a></li>
                <li><a href="{{ route('admin.nodes.view.allocation', $node->id) }}">Alocações</a></li>
                <li><a href="{{ route('admin.nodes.view.servers', $node->id) }}">Servidores</a></li>
            </ul>
        </div>
    </div>
</div>
<div class="row">
    <div class="col-sm-8">
        <div class="box box-primary">
            <div class="box-header with-border">
                <h3 class="box-title">Arquivo de Configuração</h3>
            </div>
            <div class="box-body">
                <pre class="no-margin">{{ $node->getYamlConfiguration() }}</pre>
            </div>
            <div class="box-footer">
                <p class="no-margin">Esse arquivo deve ser colocado no diretório raiz do seu daemon (geralmente <code>/etc/pterodactyl</code>) em um arquivo chamado <code>config.yml</code>.</p>
            </div>
        </div>
    </div>
    <div class="col-sm-4">
        <div class="box box-success">
            <div class="box-header with-border">
                <h3 class="box-title">Implantação automática</h3>
            </div>
            <div class="box-body">
                <p class="text-muted small">
                    Use o botão abaixo para gerar um comando de implantação personalizado que pode ser usado para configurar
                    Wings no servidor de destino com um único comando.
                </p>
            </div>
            <div class="box-footer">
                <button type="button" id="configTokenBtn" class="btn btn-sm btn-default" style="width:100%;">Gerar token</button>
            </div>
        </div>
    </div>
</div>
@endsection

@section('footer-scripts')
    @parent
    <script>
    $('#configTokenBtn').on('click', function (event) {
        $.ajax({
            method: 'POST',
            url: '{{ route('admin.nodes.view.configuration.token', $node->id) }}',
            headers: { 'X-CSRF-TOKEN': '{{ csrf_token() }}' },
        }).done(function (data) {
            swal({
                type: 'success',
                title: 'Token criado.',
                text: '<p>Para configurar automaticamente o node, execute o seguinte comando:<br /><small><pre>cd /etc/pterodactyl && sudo wings configure --panel-url {{ config('app.url') }} --token ' + data.token + ' --node ' + data.node + '{{ config('app.debug') ? ' --allow-insecure' : '' }}</pre></small></p>',
                html: true
            })
        }).fail(function () {
            swal({
                title: 'Erro',
                text: 'Algo deu errado ao criar seu token.',
                type: 'error'
            });
        });
    });
    </script>
@endsection
