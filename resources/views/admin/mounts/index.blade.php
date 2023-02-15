
@extends('layouts.admin')

@section('title')
    Montagens
@endsection

@section('content-header')
    <h1>Montagens<small>Configure e gerencie pontos de montagem adicionais para servidores.</small></h1>
    <ol class="breadcrumb">
        <li><a href="{{ route('admin.index') }}">Administrador</a></li>
        <li class="active">Montagens</li>
    </ol>
@endsection

@section('content')
    <div class="row">
        <div class="col-xs-12">
            <div class="box box-primary">
                <div class="box-header with-border">
                    <h3 class="box-title">Lista de Montagens</h3>

                    <div class="box-tools">
                        <button class="btn btn-sm btn-primary" data-toggle="modal" data-target="#newMountModal">Criar Nova</button>
                    </div>
                </div>

                <div class="box-body table-responsive no-padding">
                    <table class="table table-hover">
                        <tbody>
                            <tr>
                                <th>ID</th>
                                <th>Nome</th>
                                <th>Fonte</th>
                                <th>Alvo</th>
                                <th class="text-center">Eggs</th>
                                <th class="text-center">Nodes</th>
                                <th class="text-center">Servidores</th>
                            </tr>

                            @foreach ($mounts as $mount)
                                <tr>
                                    <td><code>{{ $mount->id }}</code></td>
                                    <td><a href="{{ route('admin.mounts.view', $mount->id) }}">{{ $mount->name }}</a></td>
                                    <td><code>{{ $mount->source }}</code></td>
                                    <td><code>{{ $mount->target }}</code></td>
                                    <td class="text-center">{{ $mount->eggs_count }}</td>
                                    <td class="text-center">{{ $mount->nodes_count }}</td>
                                    <td class="text-center">{{ $mount->servers_count }}</td>
                                </tr>
                            @endforeach
                        </tbody>
                    </table>
                </div>
            </div>
        </div>
    </div>

    <div class="modal fade" id="newMountModal" tabindex="-1" role="dialog">
        <div class="modal-dialog" role="document">
            <div class="modal-content">
                <form action="{{ route('admin.mounts') }}" method="POST">
                    <div class="modal-header">
                        <button type="button" class="close" data-dismiss="modal" aria-label="Close">
                            <span aria-hidden="true" style="color: #FFFFFF">&times;</span>
                        </button>

                        <h4 class="modal-title">Criar Montagem</h4>
                    </div>

                    <div class="modal-body">
                        <div class="row">
                            <div class="col-md-12">
                                <label for="pName" class="form-label">Nome</label>
                                <input type="text" id="pName" name="name" class="form-control" />
                                <p class="text-muted small">Nome exclusivo usado para separar esta montagem de outra.</p>
                            </div>

                            <div class="col-md-12">
                                <label for="pDescription" class="form-label">Descrição</label>
                                <textarea id="pDescription" name="description" class="form-control" rows="4"></textarea>
                                <p class="text-muted small">Uma descrição mais longa para esta montagem, deve ter menos de 191 caracteres.</p>
                            </div>

                            <div class="col-md-6">
                                <label for="pSource" class="form-label">Fonte</label>
                                <input type="text" id="pSource" name="source" class="form-control" />
                                <p class="text-muted small">Caminho do arquivo no sistema host para montar em um contêiner.</p>
                            </div>

                            <div class="col-md-6">
                                <label for="pTarget" class="form-label">Alvo</label>
                                <input type="text" id="pTarget" name="target" class="form-control" />
                                <p class="text-muted small">Onde a montagem será acessível dentro de um contêiner.</p>
                            </div>

                            <div class="col-md-6">
                                <label class="form-label">Somente leitura</label>

                                <div>
                                    <div class="radio radio-success radio-inline">
                                        <input type="radio" id="pReadOnlyFalse" name="read_only" value="0" checked>
                                        <label for="pReadOnlyFalse">Falso</label>
                                    </div>

                                    <div class="radio radio-warning radio-inline">
                                        <input type="radio" id="pReadOnly" name="read_only" value="1">
                                        <label for="pReadOnly">Verdadeiro</label>
                                    </div>
                                </div>

                                <p class="text-muted small">A montagem é lida apenas dentro do contêiner?</p>
                            </div>

                            <div class="col-md-6">
                                <label class="form-label"> Moutagem do Usuário</label>

                                <div>
                                    <div class="radio radio-success radio-inline">
                                        <input type="radio" id="pUserMountableFalse" name="user_mountable" value="0" checked>
                                        <label for="pUserMountableFalse">Falso</label>
                                    </div>

                                    <div class="radio radio-warning radio-inline">
                                        <input type="radio" id="pUserMountable" name="user_mountable" value="1">
                                        <label for="pUserMountable">Verdadeiro</label>
                                    </div>
                                </div>

                                <p class="text-muted small">Os usuários devem ser capazes de montar isso sozinhos?</p>
                            </div>
                        </div>
                    </div>

                    <div class="modal-footer">
                        {!! csrf_field() !!}
                        <button type="button" class="btn btn-default btn-sm pull-left" data-dismiss="modal">Cancelar</button>
                        <button type="submit" class="btn btn-success btn-sm">Criar</button>
                    </div>
                </form>
            </div>
        </div>
    </div>
@endsection
