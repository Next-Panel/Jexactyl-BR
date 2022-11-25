@extends('layouts.admin')

@section('title')
Servidor — {{ $server->name }}: Deletar
@endsection

@section('content-header')
    <h1>{{ $server->name }}<small>Excluir este servidor do painel.</small></h1>
    <ol class="breadcrumb">
        <li><a href="{{ route('admin.index') }}">Admin</a></li>
        <li><a href="{{ route('admin.servers') }}">Servidores</a></li>
        <li><a href="{{ route('admin.servers.view', $server->id) }}">{{ $server->name }}</a></li>
        <li class="active">Deletar</li>
    </ol>
@endsection

@section('content')
@include('admin.servers.partials.navigation')
<div class="row">
    <form id="deleteform" action="{{ route('admin.servers.view.delete', $server->id) }}" method="POST">
        <div class="col-md-6">
            <div class="box box-warning">
                <div class="box-header with-border">
                    <h3 class="box-title">Excluir servidor com segurança</h3>
                </div>
                <div class="box-body">
                    <p>Isto tentará remover o servidor do Node associado e remover os dados ligados a ele.</p>
                    <div class="checkbox checkbox-primary no-margin-bottom">
                        <input id="pReturnResourcesSafe" name="return_resources" type="checkbox" value="1" />
                        <label for="pReturnResourcesSafe">Devolver recursos ao usuário na exclusão do servidor?</label>
                    </div>
                </div>
                <div class="box-footer">
                    {!! csrf_field() !!}
                    <button id="deletebtn" class="btn btn-warning">Apagar este servidor de forma segura</button>
                </div>
            </div>
        </div>
    </form>
    <form id="forcedeleteform" action="{{ route('admin.servers.view.delete', $server->id) }}" method="POST">
        <div class="col-md-6">
            <div class="box box-danger">
                <div class="box-header with-border">
                    <h3 class="box-title">Excluir à força o servidor</h3>
                </div>
                <div class="box-body">
                    <p>Isto removerá todos os dados do servidor do Painel, independentemente de Wings ser capaz de excluir o servidor do sistema.</p>
                    <div class="checkbox checkbox-primary no-margin-bottom">
                        <input id="pReturnResources" name="return_resources" type="checkbox" value="1" />
                        <label for="pReturnResources">Devolver recursos ao usuário na exclusão do servidor?</label>
                    </div>
                </div>
                <div class="box-footer">
                    {!! csrf_field() !!}
                    <input type="hidden" name="force_delete" value="1" />
                    <button id="forcedeletebtn"" class="btn btn-danger">Excluir à força este servidor</button>
                </div>
            </div>
        </div>
    </form>
</div>
@endsection

@section('footer-scripts')
    @parent
    <script>
    $('#deletebtn').click(function (event) {
        event.preventDefault();
        swal({
            title: 'Excluir servidor',
            text: 'Todos os dados serão removidos do Painel e de seu Node.',
            showCancelButton: true,
            confirmButtonText: 'Delete',
            confirmButtonColor: 'orange',
            closeOnConfirm: false
        }, function () {
            $('#deleteform').submit()
        });
    });

    $('#forcedeletebtn').click(function (event) {
        event.preventDefault();
        swal({
            title: 'Excluir servidor',
            text: 'Todos os dados serão removidos do Painel e de seu Node.',
            showCancelButton: true,
            confirmButtonText: 'Delete',
            confirmButtonColor: 'red',
            closeOnConfirm: false
        }, function () {
            $('#forcedeleteform').submit()
        });
    });
    </script>
@endsection
